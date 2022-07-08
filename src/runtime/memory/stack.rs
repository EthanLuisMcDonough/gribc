use runtime::memory::slot::*;
use std::fmt::{Debug, Error as DebugError, Formatter};
const STACK_SIZE: usize = 5000;

pub struct Stack {
    stack_size: usize,
    pub(in runtime) stack: [StackSlot; STACK_SIZE],
}

const EMPTY_STACK_SLOT: StackSlot = StackSlot::Empty;

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
            self.stack[self.stack_size] = value;
            let ptr = self.stack_size;
            self.stack_size += 1;
            println!("PUSHED | Stack: {}", self.stack_size);
            ptr
        } else {
            panic!("Grib stack overflow");
        }
    }

    pub fn pop(&mut self) {
        if self.stack_size > 0 {
            self.stack_size -= 1;
            self.stack[self.stack_size] = StackSlot::Empty;
            println!("POPPED | Stack: {}", self.stack_size);
        }
    }

    pub fn pop_stack(&mut self, count: usize) {
        for _ in 0..count {
            self.pop();
        }
    }

    pub fn offset_slot(&'_ self, offset: usize) -> Option<&'_ StackSlot> {
        offset_calc(self.len(), offset).and_then(|ind| self.stack.get(ind))
        //self.stack.get(self.stack.len() - offset)
    }

    pub fn offset_slot_mut(&'_ mut self, offset: usize) -> Option<&'_ mut StackSlot> {
        offset_calc(self.len(), offset).and_then(move |ind| self.stack.get_mut(ind))
        //let ind = self.stack.len() - offset;
        //self.stack.get_mut(ind)
    }

    pub fn iter<'a>(&'a self) -> StackIter<'a> {
        StackIter {
            stack: self,
            index: 0,
        }
    }
}

fn offset_calc(len: usize, offset: usize) -> Option<usize> {
    len.checked_sub(offset) /*.map(|offset| {
                                print!("OFFSET: {}", offset);
                                offset
                            })*/
}

pub struct StackIter<'a> {
    stack: &'a Stack,
    index: usize,
}

impl<'a> Iterator for StackIter<'a> {
    type Item = &'a StackSlot;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.stack.stack_size {
            let ind = self.stack.stack_size - self.index - 1;
            self.index += 1;
            Some(&self.stack.stack[ind])
        } else {
            None
        }
    }
}

impl Debug for Stack {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), DebugError> {
        f.debug_list().entries(self.iter()).finish()
    }
}
