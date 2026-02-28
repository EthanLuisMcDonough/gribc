pub(in runtime) mod exec;
pub(in runtime) mod memory;
pub mod native_fn;
pub mod values;

pub use self::memory::{HeapPtr, StackOffset};
pub use self::exec::execute;
pub use self::memory::RuntimeConfig;
