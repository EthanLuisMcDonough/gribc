use ast::node::Program;
use runtime::memory::slot::*;
use runtime::values::*;
use typed_index_collections::TiVec;
use derive_more::{From, Into};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Into, Serialize, Deserialize)]
pub struct HeapPtr(pub(in runtime::memory) usize);

pub type Heap = TiVec<HeapPtr, MarkedSlot>;

#[derive(Debug, Default)]
pub struct Gc {
    pub heap: Heap,
}

impl Gc {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_captured(&'_ self, index: HeapPtr) -> Option<&'_ GribValue> {
        self.heap_slot(index).and_then(|slot| match slot {
            HeapSlot::Captured(val) => Some(val),
            _ => None,
        })
    }

    pub fn remove(&mut self, index: HeapPtr) {
        self.heap[index].value = None;
    }

    pub(in runtime::memory) fn heap_slot<'a>(&'a self, ptr: HeapPtr) -> Option<&'a HeapSlot> {
        self.heap.get(ptr).and_then(|marked| marked.value.as_ref())
    }

    pub(in runtime::memory) fn heap_slot_mut<'a>(
        &'a mut self,
        ptr: HeapPtr,
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

    pub fn heap_val_mut<'a>(&'a mut self, ptr: HeapPtr) -> Option<&'a mut HeapValue> {
        self.heap_slot_mut(ptr).and_then(|m| match m {
            MemSlot::Value(ref mut val) => Some(val),
            _ => None,
        })
    }

    pub fn heap_val<'a>(&'a self, ptr: HeapPtr) -> Option<&'a HeapValue> {
        self.heap_slot(ptr).and_then(|slot| match slot {
            HeapSlot::Value(ref val) => Some(val),
            _ => None,
        })
    }

    pub fn set_heap_val_at(&mut self, value: HeapValue, ptr: HeapPtr) {
        self.heap[ptr].value = MemSlot::Value(value).into();
    }

    pub fn get_captured_mut(&'_ mut self, index: HeapPtr) -> Option<&'_ mut GribValue> {
        self.heap_slot_mut(index).and_then(|slot| match slot {
            HeapSlot::Captured(val) => Some(val),
            _ => None,
        })
    }

    pub fn try_get_array(&'_ self, val: impl Into<GribValue>) -> Option<&'_ Vec<GribValue>> {
        let val = val.into();
        if let Some(HeapValue::Array(ref arr)) = val.ptr().and_then(|ptr| self.heap_val(ptr)) {
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
        if let Some(HeapValue::Array(ref mut arr)) =
            val.ptr().and_then(move |ptr| self.heap_val_mut(ptr))
        {
            Some(arr)
        } else {
            None
        }
    }

    pub fn try_get_hash(&'_ self, val: impl Into<GribValue>) -> Option<&'_ HashValue> {
        if let Some(HeapValue::Hash(ref h)) = val.into().ptr().and_then(|ptr| self.heap_val(ptr)) {
            Some(h)
        } else {
            None
        }
    }

    pub fn try_get_hash_mut(&'_ mut self, val: impl Into<GribValue>) -> Option<&'_ mut HashValue> {
        if let Some(HeapValue::Hash(ref mut h)) =
            val.into().ptr().and_then(move |ptr| self.heap_val_mut(ptr))
        {
            Some(h)
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

    pub fn typed_index(&self, ptr: HeapPtr) -> Option<KnownIndex> {
        self.heap_val(ptr).and_then(|v| match v {
            HeapValue::Array(_) => KnownIndex::Array(ArrayRef(ptr)).into(),
            HeapValue::Hash(_) => KnownIndex::Hash(HashRef(ptr)).into(),
            HeapValue::String(_) => KnownIndex::String(StringRef(ptr)).into(),
            _ => None,
        })
    }
}
