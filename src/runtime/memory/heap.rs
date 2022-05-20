use ast::node::Program;
use runtime::memory::{slot::*, Scope, Stack};
use runtime::values::{
    AccessFunc, Callable, GribString, GribStringRef, GribValue, HashPropertyValue, HeapValue,
};
use std::collections::{HashMap, HashSet, LinkedList};

pub struct GcConfig {
    cleanup_after: usize,
}

pub struct Gc {
    heap: Vec<MarkedSlot>,
    free_pointers: LinkedList<usize>,
    allocations: usize,
    max_allocations: usize,
}

impl Gc {
    pub fn new(config: GcConfig) -> Self {
        Gc {
            heap: Vec::new(),
            free_pointers: LinkedList::new(),
            allocations: 0,
            max_allocations: config.cleanup_after,
        }
    }

    pub fn clean(&mut self, stack: &Stack) {
        for pointer in stack.iter().flat_map(get_heap_ref) {
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

    pub(in runtime::memory) fn alloc_captured(&mut self, value: GribValue) -> usize {
        self.alloc(HeapSlot::Captured(value))
    }

    pub fn alloc_heap(&mut self, value: HeapValue) -> usize {
        self.alloc(HeapSlot::Value(value))
    }

    pub fn reserve_slot(&mut self) -> usize {
        self.alloc(HeapSlot::Empty)
    }

    pub fn capture_stack(
        &mut self,
        stack: &mut Stack,
        scope: &mut Scope,
        to_capture: &HashSet<usize>,
    ) -> Option<usize> {
        if to_capture.is_empty() {
            return None;
        }

        let mut heap_stack = HashMap::new();

        for name in to_capture {
            if let Some(ind) = scope.capture_var(stack, self, *name) {
                heap_stack.insert(*name, ind);
            }
        }

        self.alloc_heap(HeapValue::CapturedStack(heap_stack)).into()
    }

    pub fn get_captured_stack(&self, index: usize) -> Option<&HashMap<usize, usize>> {
        self.heap_slot(index).and_then(|slot| match slot {
            HeapSlot::Value(HeapValue::CapturedStack(stack)) => Some(stack),
            _ => None,
        })
    }

    pub fn get_captured(&self, index: usize) -> Option<GribValue> {
        self.heap_slot(index).and_then(|slot| match slot {
            HeapSlot::Captured(val) => Some(val.clone()),
            _ => None,
        })
    }

    fn remove(&mut self, index: usize) {
        self.heap[index].value = HeapSlot::Empty;
    }
    pub fn next_ptr(&self) -> usize {
        self.free_pointers
            .back()
            .cloned()
            .unwrap_or(self.heap.len())
    }

    fn alloc(&mut self, value: HeapSlot) -> usize {
        let value = Markable {
            value,
            marked: false,
        };

        if self.allocations > self.max_allocations {
            //self.clean();
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

    pub fn alloc_str(&mut self, s: String) -> GribString {
        GribString::Heap(self.alloc_heap(HeapValue::String(s)))
    }

    pub(in runtime::memory) fn heap_slot<'a>(&'a self, ptr: usize) -> Option<&'a HeapSlot> {
        self.heap.get(ptr).map(|marked| &marked.value)
    }

    pub(in runtime::memory) fn heap_slot_mut<'a>(
        &'a mut self,
        ptr: usize,
    ) -> Option<&'a mut HeapSlot> {
        self.heap.get_mut(ptr).map(|marked| &mut marked.value)
    }

    pub fn normalize_val(&self, val: impl Into<GribValue>) -> GribValue {
        let val = val.into();
        val.ptr()
            .and_then(|ptr| self.heap_slot(ptr))
            .and_then(|slot| match &slot {
                HeapSlot::Captured(v) => Some(v.clone()),
                _ => None,
            })
            .unwrap_or(val)
    }

    pub fn heap_val_mut<'a>(&'a mut self, ptr: usize) -> Option<&'a mut HeapValue> {
        self.heap_slot_mut(ptr).and_then(|m| match m {
            MemSlot::Value(ref mut val) => Some(val),
            _ => None,
        })
    }

    pub fn heap_val<'a>(&'a self, ptr: usize) -> Option<&'a HeapValue> {
        self.heap_slot(ptr).and_then(|slot| match slot {
            HeapSlot::Value(ref val) => Some(val),
            _ => None,
        })
    }

    /*pub fn add_captured_stack(&mut self, ptr: usize) {
        let heap = self.heap.get_mut(ptr).take();

        if let Some(HeapSlot::Value(HeapValue::CapturedStack(stack))) = &heap.map(|p| &p.value) {
            for (key, index) in stack {
                self.stack_add(StackSlot::Captured(*index));
            }
        }
    }*/

    pub fn try_get_array(&'_ self, val: GribValue) -> Option<&'_ Vec<GribValue>> {
        if let Some(HeapValue::Array(arr)) = val.ptr().and_then(|ptr| self.heap_val(ptr)) {
            Some(arr)
        } else {
            None
        }
    }

    pub fn try_get_array_mut(&'_ mut self, val: GribValue) -> Option<&'_ mut Vec<GribValue>> {
        if let Some(HeapValue::Array(arr)) = val.ptr().and_then(move |ptr| self.heap_val_mut(ptr)) {
            Some(arr)
        } else {
            None
        }
    }

    pub fn try_get_string<'a>(
        &'a self,
        val: &GribValue,
        program: &'a Program,
    ) -> Option<GribStringRef<'a>> {
        if let GribValue::String(s) = val {
            s.as_ref(program, self).into()
        } else {
            None
        }
    }
}

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
                unimplemented!()
                /*for (_, value) in &hash.values {
                    match value {
                        HashPropertyValue::Value(val) => marked_stack.push(val.clone()),
                        /*HashPropertyValue::AutoProp(prop) => {
                            for f in prop.functions() {
                                marked_func.push(f.clone());
                            }
                        }*/
                        _ => unimplemented!(),
                    }
                }*/
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
            AccessFunc::Captured(ind) => mark(gc, ind),
            AccessFunc::Callable { stack, .. } => {
                if let Some(ind) = stack {
                    mark(gc, ind);
                }
            }
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
            if let Some(ind) = binding {
                mark(gc, ind);
            }
            if let Some(ind) = stack {
                mark(gc, ind);
            }
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
