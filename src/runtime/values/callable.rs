use super::GribValue;
use ast::node::{NativeFunction, Program, ProcInd, ModuleInd, LambdaInd, GetPropInd, SetPropInd};
use runtime::{
    exec::{evaluate_lambda, run_block},
    memory::Runtime,
};
use serde::{Serialize, Deserialize};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Callable {
    Native(NativeFunction),
    Procedure {
        module: Option<ModuleInd>,
        index: ProcInd,
    },
    Lambda {
        binding: Option<usize>,
        stack: Option<usize>,
        index: LambdaInd,
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
                    &program.modules[*i].procedures[*index]
                } else {
                    &program.procedures[*index]
                };

                let alloced = runtime.add_params(&fnc.param_list, args);
                let ret = run_block(&fnc.body, &GribValue::Nil, runtime, program)
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
                let captured = runtime.add_stack(stack.clone());
                let params = runtime.add_params(&lambda.param_list, args);

                let this = binding
                    .clone()
                    .map(GribValue::HeapValue)
                    .unwrap_or_default();
                let res = evaluate_lambda(&lambda.body, &this, runtime, program);

                runtime.stack.pop_stack(captured + params);
                res
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum AccessFunc<IndexType> {
    Callable { index: IndexType, stack: Option<usize> },
    Captured(usize),
    Static(GribValue),
}

pub type GetAccessFunc = AccessFunc<GetPropInd>;
pub type SetAccessFunc = AccessFunc<SetPropInd>;
