use runtime::{
    memory::Gc,
    values::{GribValue, HeapValue},
};

#[derive(Debug)]
pub struct Markable<T> {
    pub value: T,
    pub marked: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum MemSlot<C, V> {
    Captured(C),
    Value(V),
    Empty,
}

impl<C, V> From<V> for MemSlot<C, V> {
    fn from(val: V) -> Self {
        MemSlot::Value(val)
    }
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

impl StackSlot {
    pub fn get<'a>(&'a self, gc: &'a Gc) -> Option<&'a GribValue> {
        match self {
            Self::Captured(index) => gc.get_captured(*index),
            Self::Value(val) => Some(val),
            Self::Empty => None,
        }
    }

    pub fn get_mut<'a>(&'a mut self, gc: &'a mut Gc) -> Option<&'a mut GribValue> {
        match self {
            Self::Captured(index) => gc.get_captured_mut(*index),
            Self::Value(val) => Some(val),
            Self::Empty => None,
        }
    }
}
