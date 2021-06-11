use runtime::values::{GribValue, HeapValue};
use std::collections::{HashMap, LinkedList};

const STACK_SIZE: usize = 1000;

struct Markable<T> {
    value: T,
    marked: bool,
}

#[derive(Clone, Copy)]
enum MemSlot<C, V> {
    Captured(C),
    Value(V),
    Empty,
}

type HeapSlot<'a> = MemSlot<GribValue<'a>, HeapValue<'a>>;
type MarkedSlot<'a> = Markable<HeapSlot<'a>>;
type StackSlot<'a> = MemSlot<usize, GribValue<'a>>;

fn mark<'a>(obj: &mut MarkedSlot<'a>) {
    unimplemented!();
}

fn get_heap_ref<'a>(value: &StackSlot<'a>) -> Option<usize> {
    match value {
        StackSlot::Captured(ptr) | StackSlot::Value(GribValue::HeapValue(ptr)) => Some(*ptr),
        _ => None,
    }
}

pub struct GcConfig {
    stack_size: usize,
    cleanup_after: usize,
}

pub struct Gc<'a> {
    stack: Vec<StackSlot<'a>>,
    heap: Vec<MarkedSlot<'a>>,
    free_pointers: LinkedList<usize>,
    allocations: usize,
    max_allocations: usize,
}

impl<'a> Gc<'a> {
    pub fn new(config: GcConfig) -> Self {
        Gc {
            stack: Vec::with_capacity(config.stack_size.max(STACK_SIZE)),
            heap: Vec::new(),
            free_pointers: LinkedList::new(),
            allocations: 0,
            max_allocations: config.cleanup_after,
        }
    }

    pub fn clean(&mut self) {
        for pointer in self.stack.iter().flat_map(get_heap_ref) {
            mark(&mut self.heap[pointer]);
        }

        let len = self.heap.len();
        for index in 0..len {
            if !self.heap[index].marked {
                self.remove(index);
            }

            self.heap[index].marked = false;
        }
    }

    fn alloc_captured(&mut self, value: GribValue<'a>) -> usize {
        self.alloc(HeapSlot::Captured(value))
    }

    fn alloc_heap(&mut self, value: HeapValue<'a>) -> usize {
        self.alloc(HeapSlot::Value(value))
    }

    fn remove(&mut self, index: usize) {
        self.heap[index].value = HeapSlot::Empty;
    }

    fn stack_add(&mut self, value: StackSlot<'a>) -> usize {
        let ptr = self.stack.len();
        self.stack.push(value);
        ptr
    }

    fn stack_mut(&mut self, index: usize) -> Option<&mut GribValue<'a>> {
        match self.stack.get_mut(index) {
            Some(StackSlot::Value(ref mut value)) => Some(value),
            Some(StackSlot::Captured(index)) => {
                if let Some(Markable {
                    value: HeapSlot::Captured(ref mut value),
                    ..
                }) = self.heap.get_mut(*index)
                {
                    Some(value)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn pop_stack(&mut self, count: usize) {
        for _ in 0..count {
            self.stack.pop();
        }
    }

    fn alloc(&mut self, value: HeapSlot<'a>) -> usize {
        let value = Markable {
            value,
            marked: false,
        };

        if self.allocations > self.max_allocations {
            self.clean();
            self.allocations = 0;
        }

        self.allocations += 1;

        if let Some(index) = self.free_pointers.pop_back() {
            self.heap[index] = value;
            index
        } else {
            let index = self.heap.len();
            self.heap.push(value);
            index
        }
    }
}

pub struct VariableData {
    index: usize,
    current: bool,
}

pub struct Scope<'a> {
    scope: HashMap<&'a str, usize>,
    local_count: usize,
}

impl<'a> Scope<'a> {
    pub fn new() -> Self {
        Self {
            scope: HashMap::new(),
            local_count: 0,
        }
    }

    fn declare(&mut self, label: &'a str, ptr: usize) {
        self.scope.insert(label, ptr);
        self.local_count += 1;
    }

    pub fn declare_stack(&mut self, gc: &mut Gc<'a>, label: &'a str, value: GribValue<'a>) {
        let ptr = gc.stack_add(StackSlot::Value(value));
        self.declare(label, ptr);
    }

    pub fn declare_heap(&mut self, gc: &mut Gc<'a>, label: &'a str, value: HeapValue<'a>) {
        let heap_ptr = gc.alloc_heap(value);
        let val = StackSlot::Value(GribValue::HeapValue(heap_ptr));
        let ptr = gc.stack_add(val);
        self.declare(label, ptr);
    }

    pub fn declare_captured(&mut self, gc: &mut Gc<'a>, label: &'a str, value: GribValue<'a>) {
        let heap_ptr = gc.alloc_captured(value);
        let val = StackSlot::Captured(heap_ptr);
        let ptr = gc.stack_add(val);
        self.declare(label, ptr);
    }

    pub fn cleanup(self, gc: &mut Gc<'a>) {
        gc.pop_stack(self.local_count);
    }

    fn get_mut<'b>(&self, gc: &'b mut Gc<'a>, label: &'a str) -> Option<&'b mut GribValue<'a>> {
        self.scope
            .get(label)
            .cloned()
            .and_then(move |index| gc.stack_mut(index))
    }

    pub fn set(&self, gc: &mut Gc<'a>, label: &'a str, value: GribValue<'a>) {
        if let Some(r) = self.get_mut(gc, label) {
            *r = value;
        }
    }
}

impl<'a> Clone for Scope<'a> {
    fn clone(&self) -> Self {
        Self {
            local_count: 0,
            scope: self.scope.clone(),
        }
    }
}
