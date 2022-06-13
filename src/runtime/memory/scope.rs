use ast::node::Parameters;
use runtime::memory::{slot::*, Gc, Runtime, Stack};
use runtime::values::{GribValue, HeapValue};
use std::collections::HashMap;

const STACK_OVERFLOW_MSG: &str = "Grib stack overflow";

#[derive(Debug, Clone)]
struct ScopeValue {
    stack_ptr: usize,
    global: bool,
}

#[derive(Debug)]
pub struct Scope {
    scope: HashMap<usize, ScopeValue>,
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

    fn declare(&mut self, label: usize, ptr: usize, global: bool) {
        self.scope.insert(label, ScopeValue { stack_ptr: ptr, global });
        self.local_count += 1;
    }

    pub fn declare_stack_slot(&mut self, stack: &mut Stack, label: usize, global: bool, value: impl Into<GribValue>) {
        let ptr = stack
            .add(StackSlot::Value(value.into()))
            .expect(STACK_OVERFLOW_MSG);
        self.declare(label, ptr, global);
    }

    pub fn declare_stack(&mut self, stack: &mut Stack, label: usize, value: impl Into<GribValue>) {
        self.declare_stack_slot(stack, label, false, value)
    }

    pub fn declare_heap_slot(&mut self, runtime: &mut Runtime, label: usize, global: bool, value: HeapValue) {
        let heap_ptr = runtime.alloc_heap(value);
        let val = StackSlot::Value(GribValue::HeapValue(heap_ptr));
        let ptr = runtime.stack.add(val).expect(STACK_OVERFLOW_MSG);
        self.declare(label, ptr, global);
    }

    pub fn declare_heap(&mut self, runtime: &mut Runtime, label: usize, value: HeapValue) {
        self.declare_heap_slot(runtime, label, false, value);
    }

    pub fn declare_captured(&mut self, runtime: &mut Runtime, label: usize, value: GribValue) {
        let heap_ptr = runtime.alloc_captured(value);
        let val = StackSlot::Captured(heap_ptr);
        let ptr = runtime.stack.add(val).expect(STACK_OVERFLOW_MSG);
        self.declare(label, ptr, false);
    }

    pub fn cleanup(self, stack: &mut Stack) {
        stack.pop_stack(self.local_count);
    }

    pub fn get<'a>(&self, runtime: &'a Runtime, label: usize) -> Option<&'a GribValue> {
        self.scope
            .get(&label)
            .cloned()
            .and_then(|slot| runtime.get_stack(slot.stack_ptr))
    }

    fn get_mut<'a>(&self, runtime: &'a mut Runtime, label: usize) -> Option<&'a mut GribValue> {
        self.scope
            .get(&label)
            .cloned()
            .and_then(move |slot| runtime.get_stack_mut(slot.stack_ptr))
    }

    pub fn capture_var(&mut self, runtime: &mut Runtime, label: usize) -> Option<usize> {
        self.scope.get(&label).and_then(|slot| {
            let ind = slot.stack_ptr;
            if let Some(StackSlot::Captured(new_ind)) = runtime.stack.stack.get(ind) {
                Some(*new_ind)
            } else {
                runtime.capture_at_ind(ind)
            }
        })
    }

    pub fn set(&self, runtime: &mut Runtime, label: usize, value: GribValue) {
        if let Some(r) = self.get_mut(runtime, label) {
            *r = value;
        }
    }

    pub fn add_existing_captured(&mut self, stack: &mut Stack, label: usize, index: usize) {
        let ptr = stack
            .add(StackSlot::Captured(index))
            .expect(STACK_OVERFLOW_MSG);
        self.declare(label, ptr, false);
    }

    pub fn add_captured_stack(&mut self, runtime: &mut Runtime, ptr: usize) {
        if let Some(HeapValue::CapturedStack(stack_ref)) = runtime.gc.heap_val(ptr) {
            for (key, index) in stack_ref {
                self.add_existing_captured(&mut runtime.stack, *key, *index);
            }
        }
    }

    pub fn proc_scope(&self) -> Self {
        Self {
            local_count: 0,
            scope: self.scope.clone().into_iter()
                .filter(|(_name, slot)| slot.global)
                .collect(),
            this: GribValue::Nil,
        }
    }

    pub fn add_params(&mut self, params: &Parameters, runtime: &mut Runtime, args: Vec<GribValue>) {
        let mut arg_iter = args.into_iter();

        for ident in &params.params {
            self.declare_stack(
                &mut runtime.stack,
                *ident,
                arg_iter.next().unwrap_or_default(),
            );
        }

        if let Some(ident) = &params.vardic {
            self.declare_heap(runtime, *ident, HeapValue::Array(arg_iter.collect()));
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
