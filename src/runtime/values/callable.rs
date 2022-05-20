use super::GribValue;
use ast::node::{NativeFunction, Program};
use runtime::memory::Gc;

#[derive(Clone)]
pub struct LambdaRef {
    pub binding: usize,
    index: usize,
    pub stack: usize,
}

#[derive(Clone)]
pub enum Callable {
    Native(NativeFunction),
    Procedure {
        module: Option<usize>,
        index: usize,
    },
    Lambda {
        binding: Option<usize>,
        stack: Option<usize>,
        index: usize,
    },
}

impl Callable {
    pub fn call(&self, program: &Program, gc: &mut Gc, args: Vec<GribValue>) -> GribValue {
        match self {
            Callable::Native(n) => n.call(program, gc, args),
            Callable::Procedure { module, index } => {
                let fnc = if let Some(i) = module {
                    &program.modules[*i].functions[*index]
                } else {
                    &program.functions[*index]
                };

                unimplemented!()
            }
            Callable::Lambda { .. } => {
                unimplemented!()
            }
        }
    }
}

pub struct CapturedStack {}

/*// Modules
#[derive(Clone)]
pub enum NativeReference {
    Fmt(NativeFmtPackage),
    Math(NativeMathPackage),
    Console(NativeConsolePackage),
}

impl NativeReference {
    pub fn name(&self) -> &'static str {
        unimplemented!()
    }
}

impl From<NativeFmtPackage> for NativeReference {
    fn from(f: NativeFmtPackage) -> Self {
        Self::Fmt(f)
    }
}
impl From<NativeMathPackage> for NativeReference {
    fn from(f: NativeMathPackage) -> Self {
        Self::Math(f)
    }
}
impl From<NativeConsolePackage> for NativeReference {
    fn from(f: NativeConsolePackage) -> Self {
        Self::Console(f)
    }
}*/

#[derive(Clone)]
pub enum AccessFunc {
    Callable { index: usize, stack: Option<usize> },
    Captured(usize),
}
