pub(in runtime::memory) mod heap;
mod mark;
pub(in runtime::memory) mod scope;
pub(in runtime::memory) mod slot;
pub(in runtime::memory) mod stack;

pub use self::heap::Gc;
pub use self::scope::Scope;
pub use self::stack::Stack;

use self::mark::mark;
use runtime::memory::slot::*;
use runtime::values::{GribString, GribValue, HeapValue};
use std::collections::{HashMap, HashSet};

pub struct RuntimeConfig {
    cleanup_after: usize,
}

pub struct Runtime {
    pub gc: Gc,
    pub stack: Stack,
    free_pointers: Vec<usize>,
    allocations: usize,
    max_allocations: usize,
}

impl Runtime {
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            gc: Gc::new(),
            stack: Stack::new(),
            free_pointers: Vec::new(),
            allocations: 0,
            max_allocations: config.cleanup_after,
        }
    }

    pub fn clean(&mut self) {
        for pointer in self.stack.iter().flat_map(get_heap_ref).collect::<Vec<_>>() {
            mark(self, pointer);
        }

        let len = self.gc.heap.len();
        for index in 0..len {
            if !self.gc.heap[index].marked {
                self.gc.remove(index);
            }

            self.gc.heap[index].marked = false;
        }
    }

    pub fn get_stack(&'_ self, index: usize) -> Option<&'_ GribValue> {
        match self.stack.stack.get(index) {
            Some(StackSlot::Value(value)) => Some(value),
            Some(StackSlot::Captured(index)) => {
                if let Some(HeapSlot::Captured(value)) = self.gc.heap_slot(*index) {
                    Some(value)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn get_stack_mut(&'_ mut self, index: usize) -> Option<&'_ mut GribValue> {
        match self.stack.stack.get_mut(index) {
            Some(StackSlot::Value(ref mut value)) => Some(value),
            Some(StackSlot::Captured(index)) => {
                if let Some(HeapSlot::Captured(ref mut value)) = self.gc.heap_slot_mut(*index) {
                    Some(value)
                } else {
                    None
                }
            }
            _ => None,
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

        if let Some(index) = self.free_pointers.pop() {
            self.gc.heap[index] = value;
            index
        } else {
            let index = self.gc.heap.len();
            self.gc.heap.push(value);
            index
        }
    }

    pub fn alloc_heap(&mut self, value: HeapValue) -> usize {
        self.alloc(HeapSlot::Value(value))
    }

    pub fn reserve_slot(&mut self) -> usize {
        self.alloc(HeapSlot::Empty)
    }

    pub(in runtime::memory) fn alloc_captured(&mut self, value: GribValue) -> usize {
        self.alloc(HeapSlot::Captured(value))
    }

    pub fn alloc_str(&mut self, s: String) -> GribString {
        GribString::Heap(self.alloc_heap(HeapValue::String(s)))
    }

    pub(in runtime::memory) fn capture_at_ind(&mut self, i: usize) -> Option<usize> {
        let mut heap_ind = None;

        if self.stack.len() > i {
            let mut slot = std::mem::take(&mut self.stack.stack[i]);

            if let MemSlot::Value(val) = slot {
                let ind = self.alloc_captured(val);
                slot = MemSlot::Captured(ind);
                heap_ind = ind.into();
            }

            self.stack.stack[i] = slot;
        }

        heap_ind
    }

    pub fn capture_stack(
        &mut self,
        scope: &mut Scope,
        to_capture: &HashSet<usize>,
    ) -> Option<usize> {
        if to_capture.is_empty() {
            return None;
        }

        let mut heap_stack = HashMap::new();

        for name in to_capture {
            if let Some(ind) = scope.capture_var(self, *name) {
                heap_stack.insert(*name, ind);
            }
        }

        self.alloc_heap(HeapValue::CapturedStack(heap_stack)).into()
    }
}

fn get_heap_ref<'a>(value: &StackSlot) -> Option<usize> {
    match value {
        StackSlot::Captured(ptr) | StackSlot::Value(GribValue::HeapValue(ptr)) => Some(*ptr),
        _ => None,
    }
}
