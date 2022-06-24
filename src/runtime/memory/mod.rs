pub(in runtime::memory) mod heap;
mod mark;
pub(in runtime::memory) mod scope;
pub(in runtime::memory) mod slot;
pub(in runtime::memory) mod stack;

pub use self::heap::Gc;
pub use self::scope::Scope;
pub use self::slot::StackSlot;
pub use self::stack::Stack;

use self::mark::*;
use runtime::memory::slot::*;
use runtime::values::{GribString, GribValue, HeapValue};
use std::collections::{HashMap, HashSet};

pub struct RuntimeConfig {
    pub cleanup_after: usize,
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
        for slot in self.stack.iter() {
            match slot {
                StackSlot::Captured(ind) => mark_heap(&mut self.gc, *ind),
                StackSlot::Value(val) => mark_stack(&mut self.gc, val),
                StackSlot::Empty => {}
            }
        }

        let len = self.gc.heap.len();
        for index in 0..len {
            if !self.gc.heap[index].marked {
                self.gc.remove(index);
                self.free_pointers.push(index);
            }

            self.gc.heap[index].marked = false;
        }
    }

    pub fn get_stack(&'_ self, index: usize) -> Option<&'_ GribValue> {
        match self.stack.stack.get(index) {
            Some(StackSlot::Value(value)) => Some(value),
            Some(StackSlot::Captured(index)) => self.gc.get_captured(*index),
            _ => None,
        }
    }

    pub fn get_stack_mut(&'_ mut self, index: usize) -> Option<&'_ mut GribValue> {
        match self.stack.stack.get_mut(index) {
            Some(StackSlot::Value(ref mut value)) => Some(value),
            Some(StackSlot::Captured(index)) => self.gc.get_captured_mut(*index),
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
        if s.is_empty() {
            GribString::Static("")
        } else if s.len() == 1 {
            GribString::Char(s.chars().next().unwrap())
        } else {
            GribString::Heap(self.alloc_heap(HeapValue::String(s)))
        }
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
            if let Some(slot) = scope.get_slot(&self.stack, *name) {
                heap_stack.insert(*name, slot.clone());
            }
        }

        self.alloc_heap(HeapValue::CapturedStack(heap_stack)).into()
    }
}
