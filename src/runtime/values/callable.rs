use super::GribValue;
use ast::node::{NativeFunction, Program};
use runtime::{
    exec::run_block,
    memory::{Runtime, Scope},
};

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
    pub fn call(
        &self,
        program: &Program,
        runtime: &mut Runtime,
        args: Vec<GribValue>,
    ) -> GribValue {
        match self {
            Callable::Native(n) => n.call(program, runtime, args),
            Callable::Procedure { module, index } => {
                let fnc = if let Some(i) = module {
                    &program.modules[*i].functions[*index]
                } else {
                    &program.functions[*index]
                };

                let mut scope = Scope::new();
                scope.add_params(&fnc.param_list, runtime, args);

                run_block(&fnc.body, &mut scope, runtime, program)
                    .map(GribValue::from)
                    .unwrap_or_default()
            }
            Callable::Lambda { .. } => {
                unimplemented!()
            }
        }
    }
}

#[derive(Clone)]
pub enum AccessFunc {
    Callable { index: usize, stack: Option<usize> },
    Captured(usize),
}
