use super::GribValue;
use ast::node::{NativeFunction, Program};
use runtime::{
    exec::{evaluate_lambda, run_block, LocalState},
    memory::Runtime,
};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
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

                let state = LocalState::default();
                let alloced = runtime.add_params(&fnc.param_list, args);
                let ret = run_block(&fnc.body, &state, runtime, program)
                    .map(GribValue::from)
                    .unwrap_or_default();

                runtime.stack.pop_stack(alloced);
                ret
            }
            Callable::Lambda {
                binding,
                stack,
                index,
            } => {
                let lambda = &program.lambdas[*index];
                let params = runtime.add_params(&lambda.param_list, args);

                let this = binding
                    .clone()
                    .map(GribValue::HeapValue)
                    .unwrap_or_default();
                let state = LocalState::new(this, stack.clone());

                let res = evaluate_lambda(&lambda.body, &state, runtime, program);
                runtime.stack.pop_stack(params);

                res
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum AccessFunc {
    Callable { index: usize, stack: Option<usize> },
    Captured(usize),
    Static(GribValue),
}
