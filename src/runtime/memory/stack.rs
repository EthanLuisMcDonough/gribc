use runtime::{memory::StackSlot, values::GribValue};
use std::fmt::{Debug, Error as DebugError, Formatter};

const STACK_SIZE: usize = 5000;
const CALL_EMPTY_THIS: &'static str = "ATTEMPT TO LOAD THIS WITH EMPTY STACK";
const CALL_EMPTY_STACK: &'static str = "ATTEMPT TO LOAD STACK WITH EMPTY STACK";

#[derive(Clone, Default)]
struct LocalState {
    this: GribValue,
    lambda: Option<usize>,
}

impl LocalState {
    fn new(this: GribValue, lambda: Option<usize>) -> Self {
        let this = this.into();
        Self { this, lambda }
    }
}

pub struct Stack {
    stack_size: usize,
    pub(in runtime) stack: [Option<StackSlot>; STACK_SIZE],
    call_stack: Vec<LocalState>,
}

const EMPTY_SLOT: Option<StackSlot> = None;

impl Stack {
    pub fn new() -> Self {
        Self {
            stack_size: 0,
            stack: [EMPTY_SLOT; STACK_SIZE],
            call_stack: Vec::new(),
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

    fn get_ind(&self, ind: usize) -> Option<&StackSlot> {
        match self.stack.get(ind) {
            Some(Some(slot)) => Some(slot),
            _ => None,
        }
    }

    fn get_ind_mut(&mut self, ind: usize) -> Option<&mut StackSlot> {
        match self.stack.get_mut(ind) {
            Some(Some(slot)) => Some(slot),
            _ => None,
        }
    }

    pub fn offset_slot(&'_ self, offset: usize) -> Option<&'_ StackSlot> {
        offset_calc(self.len(), offset).and_then(|ind| self.get_ind(ind))
    }

    pub fn offset_slot_mut(&'_ mut self, offset: usize) -> Option<&'_ mut StackSlot> {
        offset_calc(self.len(), offset).and_then(move |ind| self.get_ind_mut(ind))
    }

    pub fn iter<'a>(&'a self) -> StackIter<'a> {
        StackIter {
            stack: self,
            index: 0,
        }
    }

    pub fn add_call(&mut self, this: GribValue, lambda: Option<usize>) {
        self.call_stack.push(LocalState::new(this, lambda));
    }

    pub fn pop_call(&mut self) {
        self.call_stack.pop();
    }

    pub fn get_this(&self) -> GribValue {
        self.call_stack.last().expect(CALL_EMPTY_THIS).this.clone()
    }

    pub fn get_call_stack(&self) -> Option<usize> {
        self.call_stack
            .last()
            .expect(CALL_EMPTY_STACK)
            .lambda
            .clone()
    }
}

fn offset_calc(len: usize, offset: usize) -> Option<usize> {
    len.checked_sub(offset)
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
            self.stack.stack[ind].as_ref()
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
