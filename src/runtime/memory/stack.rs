use runtime::memory::slot::*;
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

    pub fn add(&mut self, value: StackSlot) -> Option<usize> {
        if self.stack_size < STACK_SIZE {
            self.stack[self.stack_size] = value;
            let ptr = self.stack_size;
            self.stack_size += 1;
            Some(ptr)
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

    pub fn iter<'a>(&'a self) -> StackIter<'a> {
        StackIter {
            stack: self,
            index: 0,
        }
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
