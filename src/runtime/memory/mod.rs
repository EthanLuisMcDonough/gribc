pub(in runtime::memory) mod heap;
pub(in runtime::memory) mod scope;
pub(in runtime::memory) mod slot;
pub(in runtime::memory) mod stack;

pub use self::heap::{Gc, GcConfig};
pub use self::scope::Scope;
pub use self::stack::Stack;
