use runtime::values::{GribValue, HeapValue};

#[derive(Debug)]
pub struct Markable<T> {
    pub value: T,
    pub marked: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum MemSlot<C, V> {
    Captured(C),
    Value(V),
}

impl<C, V> From<V> for MemSlot<C, V> {
    fn from(val: V) -> Self {
        MemSlot::Value(val)
    }
}

impl<C, V> MemSlot<C, V> {
    pub fn is_value(&self) -> bool {
        match self {
            MemSlot::Value(_) => true,
            _ => false,
        }
    }
}

pub type HeapSlot = MemSlot<GribValue, HeapValue>;
pub type MarkedSlot = Markable<Option<HeapSlot>>;
pub type StackSlot = MemSlot<usize, GribValue>;
