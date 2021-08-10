use runtime::values::{AccessFunc, Callable, GribValue, HashPropertyValue, HeapValue};
use std::collections::{HashMap, HashSet, LinkedList};
use std::mem;

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

impl<C, V> Default for MemSlot<C, V> {
    fn default() -> Self {
        Self::Empty
    }
}

impl<C, V> MemSlot<C, V> {
    fn is_value(&self) -> bool {
        match self {
            MemSlot::Value(_) => true,
            _ => false,
        }
    }
}

type HeapSlot = MemSlot<GribValue, HeapValue>;
type MarkedSlot = Markable<HeapSlot>;
type StackSlot = MemSlot<usize, GribValue>;

fn mark(gc: &mut Gc, ind: usize) {
    let Markable { marked, ref value } = &mut gc.heap[ind];

    if *marked {
        return;
    }

    let mut to_mark = true;
    let mut marked_stack = Vec::new();
    let mut marked_heap = Vec::new();
    let mut marked_func = Vec::new();

    match value {
        HeapSlot::Captured(val) => marked_stack.push(val.clone()), // shouldn't recurse deeper than one level
        HeapSlot::Value(val) => match val {
            HeapValue::String(_) => {}
            HeapValue::Array(arr) => {
                for i in arr.iter() {
                    marked_stack.push(i.clone());
                }
            }
            HeapValue::CapturedStack(stack) => {
                for (_, index) in stack.iter() {
                    marked_heap.push(*index);
                }
            }
            HeapValue::Hash(hash) => {
                for (_, value) in &hash.values {
                    match value {
                        HashPropertyValue::Value(val) => marked_stack.push(val.clone()),
                        HashPropertyValue::AutoProp(prop) => {
                            for f in prop.functions() {
                                marked_func.push(f.clone());
                            }
                        }
                    }
                }
            }
        },
        _ => to_mark = false,
    }

    *marked = to_mark;

    for value in marked_stack {
        mark_stack(gc, value);
    }

    for value in marked_heap {
        mark(gc, value);
    }

    for value in marked_func {
        match value {
            AccessFunc::Callable(c) => mark_function(gc, c),
            AccessFunc::Captured(ind) => mark(gc, ind),
        }
    }
}

fn mark_stack(gc: &mut Gc, obj: GribValue) {
    match obj {
        GribValue::HeapValue(heap) => mark(gc, heap),
        GribValue::Callable(callable) => mark_function(gc, callable),
        _ => {}
    }
}

fn mark_function(gc: &mut Gc, fnc: Callable) {
    match fnc {
        Callable::Lambda { binding, stack, .. } => {
            mark(gc, binding);
            mark(gc, stack);
        }
        _ => {}
    }
}

fn get_heap_ref<'a>(value: &StackSlot) -> Option<usize> {
    match value {
        StackSlot::Captured(ptr) | StackSlot::Value(GribValue::HeapValue(ptr)) => Some(*ptr),
        _ => None,
    }
}

pub struct GcConfig {
    stack_size: usize,
    cleanup_after: usize,
}

pub struct Gc {
    stack: Vec<StackSlot>,
    heap: Vec<MarkedSlot>,
    free_pointers: LinkedList<usize>,
    allocations: usize,
    max_allocations: usize,
}

impl Gc {
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
        for pointer in self.stack.iter().flat_map(get_heap_ref).collect::<Vec<_>>() {
            mark(self, pointer);
        }

        let len = self.heap.len();
        for index in 0..len {
            if !self.heap[index].marked {
                self.remove(index);
            }

            self.heap[index].marked = false;
        }
    }

    fn alloc_captured(&mut self, value: GribValue) -> usize {
        self.alloc(HeapSlot::Captured(value))
    }

    pub fn alloc_heap(&mut self, value: HeapValue) -> usize {
        self.alloc(HeapSlot::Value(value))
    }

    pub fn capture_stack(&mut self, scope: &mut Scope, to_capture: &HashSet<String>) -> usize {
        let mut stack = HashMap::new();

        for name in to_capture {
            if let Some(ind) = scope.capture_var(self, name) {
                stack.insert(name.clone(), ind);
            }
        }

        self.alloc_heap(HeapValue::CapturedStack(stack))
    }

    fn remove(&mut self, index: usize) {
        self.heap[index].value = HeapSlot::Empty;
    }

    fn stack_add(&mut self, value: StackSlot) -> usize {
        let ptr = self.stack.len();
        self.stack.push(value);
        ptr
    }

    fn stack_mut(&mut self, index: usize) -> Option<&mut GribValue> {
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

    fn alloc(&mut self, value: HeapSlot) -> usize {
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

    pub fn alloc_str(&mut self, s: String) -> GribValue {
        GribValue::HeapValue(self.alloc_heap(HeapValue::String(s)))
    }

    pub fn get_str<'a>(&'a self, i: impl Into<GribValue>) -> Option<&'a String> {
        self.heap_val(i.into()).and_then(|v| match v {
            HeapValue::String(s) => Some(s),
            _ => None,
        })
    }

    pub fn normalize_val(&self, val: GribValue) -> GribValue {
        val.ptr()
            .and_then(|ind| self.heap.get(ind))
            .and_then(|m| match &m.value {
                MemSlot::Captured(v) => Some(v.clone()),
                _ => None,
            })
            .unwrap_or(val)
    }

    pub fn heap_val_mut<'a>(&'a mut self, val: GribValue) -> Option<&'a mut HeapValue> {
        val.ptr()
            .and_then(move |ind| self.heap.get_mut(ind))
            .and_then(|m| match m.value {
                MemSlot::Value(ref mut val) => Some(val),
                _ => None,
            })
    }

    pub fn heap_val<'a>(&'a self, val: impl Into<GribValue>) -> Option<&'a HeapValue> {
        val.into()
            .ptr()
            .and_then(move |ind| self.heap.get(ind))
            .and_then(|m| match m.value {
                MemSlot::Value(ref val) => Some(val),
                _ => None,
            })
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

    pub fn declare_stack(&mut self, gc: &mut Gc, label: &'a str, value: impl Into<GribValue>) {
        let ptr = gc.stack_add(StackSlot::Value(value.into()));
        self.declare(label, ptr);
    }

    pub fn declare_heap(&mut self, gc: &mut Gc, label: &'a str, value: HeapValue) {
        let heap_ptr = gc.alloc_heap(value);
        let val = StackSlot::Value(GribValue::HeapValue(heap_ptr));
        let ptr = gc.stack_add(val);
        self.declare(label, ptr);
    }

    pub fn declare_captured(&mut self, gc: &mut Gc, label: &'a str, value: GribValue) {
        let heap_ptr = gc.alloc_captured(value);
        let val = StackSlot::Captured(heap_ptr);
        let ptr = gc.stack_add(val);
        self.declare(label, ptr);
    }

    pub fn cleanup(self, gc: &mut Gc) {
        gc.pop_stack(self.local_count);
    }

    fn get_mut<'b>(&self, gc: &'b mut Gc, label: &str) -> Option<&'b mut GribValue> {
        self.scope
            .get(label)
            .cloned()
            .and_then(move |index| gc.stack_mut(index))
    }

    fn capture_var(&mut self, gc: &mut Gc, label: &str) -> Option<usize> {
        let mut heap_ind = None;

        if let Some(&ind) = self.scope.get(label) {
            let mut slot = mem::take(&mut gc.stack[ind]);

            if let MemSlot::Value(val) = slot {
                let heap_ind = gc.alloc_captured(val);
                slot = MemSlot::Captured(heap_ind);
            }

            heap_ind = ind.into();
            gc.stack[ind] = slot;
        }

        heap_ind
    }

    pub fn set(&self, gc: &mut Gc, label: &str, value: GribValue) {
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
