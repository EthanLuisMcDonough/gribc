use runtime::{
    memory::Gc,
    values::{GribValue, HeapValue},
};

#[derive(Debug, Default)]
pub struct Markable<T> {
    pub value: Option<T>,
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

pub type HeapSlot = MemSlot<GribValue, HeapValue>;
pub type MarkedSlot = Markable<HeapSlot>;
pub type StackSlot = MemSlot<usize, GribValue>;

impl StackSlot {
    pub fn get<'a>(&'a self, gc: &'a Gc) -> &'a GribValue {
        match self {
            Self::Captured(index) => gc.get_captured(*index),
            Self::Value(val) => val,
        }
    }

    pub fn get_mut<'a>(&'a mut self, gc: &'a mut Gc) -> &'a mut GribValue {
        match self {
            Self::Captured(index) => gc.get_captured_mut(*index),
            Self::Value(val) => val,
        }
    }
}
