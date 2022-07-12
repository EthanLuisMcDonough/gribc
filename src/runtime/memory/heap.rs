use ast::node::Program;
use runtime::memory::slot::*;
use runtime::values::*;

#[derive(Debug)]
pub struct Gc {
    pub heap: Vec<MarkedSlot>,
}

impl Gc {
    pub fn new() -> Self {
        Gc { heap: Vec::new() }
    }

    pub fn get_captured(&self, index: usize) -> &GribValue {
        if let Some(HeapSlot::Captured(val)) = self.heap_slot(index) {
            val
        } else {
            panic!("COULD NOT READ CAPTURED VARIABLE AT {}", index);
        }
    }

    pub fn get_captured_mut(&mut self, index: usize) -> &mut GribValue {
        if let Some(HeapSlot::Captured(val)) = self.heap_slot_mut(index) {
            val
        } else {
            panic!("COULD NOT READ MUTABLE CAPTURED VARIABLE AT {}", index);
        }
    }

    pub fn remove(&mut self, index: usize) {
        if let Some(Markable { marked, value }) = self.heap.get_mut(index) {
            *marked = false;
            *value = None;
        }
    }

    pub(in runtime::memory) fn heap_slot<'a>(&'a self, ptr: usize) -> Option<&'a HeapSlot> {
        self.heap.get(ptr).and_then(|marked| marked.value.as_ref())
    }

    pub(in runtime::memory) fn heap_slot_mut<'a>(
        &'a mut self,
        ptr: usize,
    ) -> Option<&'a mut HeapSlot> {
        self.heap
            .get_mut(ptr)
            .and_then(|marked| marked.value.as_mut())
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
        if let Some(slot) = self.heap.get_mut(ptr) {
            slot.value = Some(MemSlot::Value(value));
        }
    }

    pub fn try_get_array(&'_ self, val: impl Into<GribValue>) -> Option<&'_ Vec<GribValue>> {
        let val = val.into();
        if let Some(HeapValue::Array(arr)) = val.ptr().and_then(|ptr| self.heap_val(ptr)) {
            Some(arr)
        } else {
            None
        }
    }

    pub fn try_get_array_mut(
        &'_ mut self,
        val: impl Into<GribValue>,
    ) -> Option<&'_ mut Vec<GribValue>> {
        let val = val.into();
        if let Some(HeapValue::Array(arr)) = val.ptr().and_then(move |ptr| self.heap_val_mut(ptr)) {
            Some(arr)
        } else {
            None
        }
    }

    pub fn try_get_hash(&'_ self, val: impl Into<GribValue>) -> Option<&'_ HashValue> {
        if let Some(HeapValue::Hash(h)) = val.into().ptr().and_then(|ptr| self.heap_val(ptr)) {
            Some(h)
        } else {
            None
        }
    }

    pub fn try_get_hash_mut(&'_ mut self, val: impl Into<GribValue>) -> Option<&'_ mut HashValue> {
        if let Some(HeapValue::Hash(h)) =
            val.into().ptr().and_then(move |ptr| self.heap_val_mut(ptr))
        {
            Some(h)
        } else {
            None
        }
    }

    pub fn try_get_stack(&'_ self, ind: usize) -> Option<&'_ Vec<StackSlot>> {
        if let Some(HeapValue::CapturedStack(stack)) = self.heap_val(ind) {
            Some(stack)
        } else {
            None
        }
    }

    pub fn try_get_stack_mut(&mut self, ind: usize) -> Option<&mut Vec<StackSlot>> {
        if let Some(HeapValue::CapturedStack(stack)) = self.heap_val_mut(ind) {
            Some(stack)
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

    pub fn typed_index(&self, ptr: usize) -> Option<KnownIndex> {
        self.heap_val(ptr).and_then(|v| match v {
            HeapValue::Array(_) => KnownIndex::Array(ArrayRef(ptr)).into(),
            HeapValue::Hash(_) => KnownIndex::Hash(HashRef(ptr)).into(),
            HeapValue::String(_) => KnownIndex::String(StringRef(ptr)).into(),
            _ => None,
        })
    }
}
