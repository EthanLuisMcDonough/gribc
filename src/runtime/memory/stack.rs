use runtime::memory::slot::*;
use std::fmt::{Debug, Error as DebugError, Formatter};
const STACK_SIZE: usize = 5000;

pub struct Stack {
    stack_size: usize,
    stack: [Option<StackSlot>; STACK_SIZE],
}

const EMPTY_STACK_SLOT: Option<StackSlot> = None;

impl Stack {
    pub fn new() -> Self {
        Self {
            stack_size: 0,
            stack: [EMPTY_STACK_SLOT; STACK_SIZE],
        }
    }

    pub fn len(&self) -> usize {
        self.stack_size
    }

    pub fn add(&mut self, value: impl Into<StackSlot>) -> usize {
        let value = value.into();
        if self.stack_size < STACK_SIZE {
            self.stack[self.stack_size] = Some(value);
            let ptr = self.stack_size;
            self.stack_size += 1;
            ptr
        } else {
            panic!("Grib stack overflow");
        }
    }

    pub fn pop(&mut self) {
        if self.stack_size > 0 {
            self.stack_size -= 1;
            self.stack[self.stack_size] = None;
        }
    }

    pub fn pop_stack(&mut self, count: usize) {
        for _ in 0..count {
            self.pop();
        }
    }

    fn offset_calc(&self, offset: usize) -> Option<usize> {
        self.len().checked_sub(offset)
    }

    pub fn offset_slot(&'_ self, offset: usize) -> Option<&'_ StackSlot> {
        self.offset_calc(offset)
            .and_then(|ind| self.stack.get(ind))
            .and_then(|o| o.as_ref())
    }

    pub fn offset_slot_mut(&'_ mut self, offset: usize) -> Option<&'_ mut StackSlot> {
        self.offset_calc(offset)
            .and_then(move |ind| self.stack.get_mut(ind))
            .and_then(|o| o.as_mut())
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a StackSlot> {
        self.stack
            .iter()
            .take(self.stack_size)
            .filter_map(|s| s.as_ref())
    }
}

impl Debug for Stack {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), DebugError> {
        f.debug_list().entries(self.iter()).finish()
    }
}
