use runtime::memory::slot::*;
use runtime::memory::Gc;
use runtime::values::GribValue;
use std::mem::take;
const STACK_SIZE: usize = 10000;

pub struct Stack {
    stack_size: usize,
    stack: [StackSlot; STACK_SIZE],
}

const EMPTY_STACK_SLOT: StackSlot = StackSlot::Empty;

impl Stack {
    pub fn new() -> Self {
        Self {
            stack_size: 0,
            stack: [EMPTY_STACK_SLOT; STACK_SIZE],
        }
    }

    pub fn add(&mut self, value: StackSlot) -> Option<usize> {
        if self.stack_size < STACK_SIZE {
            self.stack[self.stack_size] = value;
            self.stack_size += 1;
            Some(self.stack_size)
        } else {
            None
        }
    }

    pub fn pop(&mut self) {
        if self.stack_size > 0 {
            self.stack_size -= 1;
            self.stack[self.stack_size] = StackSlot::Empty;
        }
    }

    pub fn pop_stack(&mut self, count: usize) {
        for _ in 0..count {
            self.pop();
        }
    }

    pub fn get<'a>(&'a self, gc: &'a Gc, index: usize) -> Option<&'a GribValue> {
        match self.stack.get(index) {
            Some(StackSlot::Value(value)) => Some(value),
            Some(StackSlot::Captured(index)) => {
                if let Some(HeapSlot::Captured(value)) = gc.heap_slot(*index) {
                    Some(value)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn get_mut<'a>(&'a mut self, gc: &'a mut Gc, index: usize) -> Option<&'a mut GribValue> {
        match self.stack.get_mut(index) {
            Some(StackSlot::Value(ref mut value)) => Some(value),
            Some(StackSlot::Captured(index)) => {
                if let Some(HeapSlot::Captured(ref mut value)) = gc.heap_slot_mut(*index) {
                    Some(value)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn iter<'a>(&'a self) -> StackIter<'a> {
        StackIter {
            stack: self,
            index: 0,
        }
    }

    pub(in runtime::memory) fn capture_at_ind(&mut self, i: usize, gc: &mut Gc) -> Option<usize> {
        let mut heap_ind = None;

        if self.stack_size > i {
            let mut slot = take(&mut self.stack[i]);

            if let MemSlot::Value(val) = slot {
                let ind = gc.alloc_captured(val);
                slot = MemSlot::Captured(ind);
                heap_ind = ind.into();
            }

            self.stack[i] = slot;
        }

        heap_ind
    }
}

pub struct StackIter<'a> {
    stack: &'a Stack,
    index: usize,
}

impl<'a> Iterator for StackIter<'a> {
    type Item = &'a StackSlot;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.stack.stack_size {
            let ind = self.stack.stack_size - self.index;
            self.index += 1;
            Some(&self.stack.stack[ind])
        } else {
            None
        }
    }
}
