use super::{GribValue, HashValue};
use runtime::memory::{Gc, StackSlot};

/*macro_rules! try_method {
    ($n:ident, $e:expr) => {
        if let Self::$n(v) = $e {
            Some(v)
        } else {
            None
        }
    };
}*/

#[derive(Clone, Debug)]
pub enum HeapValue {
    Array(Vec<GribValue>),
    Hash(HashValue),
    String(String),
    CapturedStack(Vec<StackSlot>),
}

macro_rules! type_ref {
    ($name:ident $heap_name:ident $inner_type:ty) => {
        pub struct $name(pub usize);

        impl $name {
            pub fn get<'a>(&self, gc: &'a Gc) -> Option<&'a $inner_type> {
                if let Some(HeapValue::$heap_name(h)) = gc.heap_val(self.0) {
                    Some(h)
                } else {
                    None
                }
            }

            pub fn ptr(&self) -> usize {
                self.0
            }
        }
    };
}

// For use when the type needs to be determined before the actual value is referenced
// Useful for when runtime needs to pre-process something
// e.g. indexaccess
type_ref!(HashRef Hash HashValue);
type_ref!(ArrayRef Array Vec<GribValue>);
type_ref!(StringRef String String);
pub enum KnownIndex {
    Hash(HashRef),
    Array(ArrayRef),
    String(StringRef),
}
