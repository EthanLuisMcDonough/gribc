use ast::node::Program;
use runtime::memory::slot::*;
use runtime::values::{GribStringRef, GribValue, HeapValue};
use std::collections::HashMap;

pub struct Gc {
    pub(in runtime::memory) heap: Vec<MarkedSlot>,
}

impl Gc {
    pub fn new() -> Self {
        Gc { heap: Vec::new() }
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

    pub fn remove(&mut self, index: usize) {
        self.heap[index].value = HeapSlot::Empty;
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

    pub fn set_heap_val_at(&mut self, value: HeapValue, ptr: usize) {
        if let Some(slot) = self.heap_slot_mut(ptr) {
            *slot = MemSlot::Value(value);
        }
    }

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
