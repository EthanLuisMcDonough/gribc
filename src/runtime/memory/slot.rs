use runtime::values::{GribValue, HeapValue};

pub struct Markable<T> {
    pub value: T,
    pub marked: bool,
}

#[derive(Clone, Copy)]
pub enum MemSlot<C, V> {
    Captured(C),
    Value(V),
    Empty,
}

impl<C, V> Default for MemSlot<C, V> {
    fn default() -> Self {
        Self::Empty
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
pub type MarkedSlot = Markable<HeapSlot>;
pub type StackSlot = MemSlot<usize, GribValue>;
