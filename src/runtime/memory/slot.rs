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

const EMPTY_ACCESS_ATTEMPT: &'static str = "ATTEMPT TO ACCESS EMPTY STACK SLOT";

impl StackSlot {
    pub fn get<'a>(&'a self, gc: &'a Gc) -> &'a GribValue {
        match self {
            Self::Captured(index) => gc.get_captured(*index),
            Self::Value(val) => val,
            Self::Empty => panic!(EMPTY_ACCESS_ATTEMPT),
        }
    }

    pub fn get_mut<'a>(&'a mut self, gc: &'a mut Gc) -> &'a mut GribValue {
        match self {
            Self::Captured(index) => gc.get_captured_mut(*index),
            Self::Value(val) => val,
            Self::Empty => panic!(EMPTY_ACCESS_ATTEMPT),
        }
    }
}
