use runtime::memory::{slot::*, Gc, Stack};
use runtime::values::{GribValue, HeapValue};
use std::collections::HashMap;

const STACK_OVERFLOW_MSG: &str = "Grib stack overflow";

pub struct Scope {
    scope: HashMap<usize, usize>,
    local_count: usize,
    this: GribValue,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            scope: HashMap::new(),
            local_count: 0,
            this: GribValue::Nil,
        }
    }

    pub fn get_this(&self, gc: &Gc) -> GribValue {
        gc.normalize_val(self.this.clone())
    }

    pub fn set_this(&mut self, this: impl Into<GribValue>) {
        self.this = this.into();
    }

    fn declare(&mut self, label: usize, ptr: usize) {
        self.scope.insert(label, ptr);
        self.local_count += 1;
    }

    pub fn declare_stack(&mut self, stack: &mut Stack, label: usize, value: impl Into<GribValue>) {
        let ptr = stack
            .add(StackSlot::Value(value.into()))
            .expect(STACK_OVERFLOW_MSG);
        self.declare(label, ptr);
    }

    pub fn declare_heap(&mut self, stack: &mut Stack, gc: &mut Gc, label: usize, value: HeapValue) {
        let heap_ptr = gc.alloc_heap(value);
        let val = StackSlot::Value(GribValue::HeapValue(heap_ptr));
        let ptr = stack.add(val).expect(STACK_OVERFLOW_MSG);
        self.declare(label, ptr);
    }

    pub fn declare_captured(
        &mut self,
        stack: &mut Stack,
        gc: &mut Gc,
        label: usize,
        value: GribValue,
    ) {
        let heap_ptr = gc.alloc_captured(value);
        let val = StackSlot::Captured(heap_ptr);
        let ptr = stack.add(val).expect(STACK_OVERFLOW_MSG);
        self.declare(label, ptr);
    }

    pub fn cleanup(self, stack: &mut Stack) {
        stack.pop_stack(self.local_count);
    }

    pub fn get<'a>(&self, stack: &'a Stack, gc: &'a Gc, label: usize) -> Option<&'a GribValue> {
        self.scope
            .get(&label)
            .cloned()
            .and_then(|index| stack.get(gc, index))
    }

    fn get_mut<'a>(
        &self,
        stack: &'a mut Stack,
        gc: &'a mut Gc,
        label: usize,
    ) -> Option<&'a mut GribValue> {
        self.scope
            .get(&label)
            .cloned()
            .and_then(move |index| stack.get_mut(gc, index))
    }

    pub fn capture_var(&mut self, stack: &mut Stack, gc: &mut Gc, label: usize) -> Option<usize> {
        self.scope
            .get(&label)
            .and_then(|&ind| stack.capture_at_ind(ind, gc))
    }

    pub fn set(&self, stack: &mut Stack, gc: &mut Gc, label: usize, value: GribValue) {
        if let Some(r) = self.get_mut(stack, gc, label) {
            *r = value;
        }
    }

    pub fn add_existing_captured(&mut self, stack: &mut Stack, label: usize, index: usize) {
        let ptr = stack
            .add(StackSlot::Captured(index))
            .expect(STACK_OVERFLOW_MSG);
        self.declare(label, ptr);
    }

    pub fn add_captured_stack(&mut self, stack: &mut Stack, gc: &mut Gc, ptr: usize) {
        if let Some(HeapValue::CapturedStack(stack_ref)) = gc.heap_val(ptr) {
            for (key, index) in stack_ref {
                self.add_existing_captured(stack, *key, *index);
            }
        }
    }
}

impl Clone for Scope {
    fn clone(&self) -> Self {
        Self {
            local_count: 0,
            scope: self.scope.clone(),
            this: self.this.clone(),
        }
    }
}
