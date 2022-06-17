use super::GribValue;
use ast::node::{NativeFunction, Program};
use runtime::{
    exec::{evaluate_lambda, run_block},
    memory::{Runtime, Scope},
};

#[derive(Clone)]
pub struct LambdaRef {
    pub binding: usize,
    index: usize,
    pub stack: usize,
}

#[derive(Clone, PartialEq, Debug)]
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

                let mut scope = module
                    .as_ref()
                    .map(|&ind| {
                        let module = &program.modules[ind];
                        let mut sc = Scope::new();
                        sc.scope_imports(runtime, program, &module.imports);
                        sc.scope_functions(runtime, &module.functions, Some(ind));
                        sc
                    })
                    .unwrap_or(runtime.base_scope.clone());
                scope.add_params(&fnc.param_list, runtime, args);

                run_block(&fnc.body, scope, runtime, program)
                    .map(GribValue::from)
                    .unwrap_or_default()
            }
            Callable::Lambda {
                binding,
                stack,
                index,
            } => {
                let lambda = &program.lambdas[*index];
                let mut scope = Scope::new();

                if let Some(stack_ptr) = stack {
                    scope.add_captured_stack(runtime, *stack_ptr);
                }
                scope.add_params(&lambda.param_list, runtime, args);

                evaluate_lambda(&lambda.body, scope, binding.clone(), runtime, program)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum AccessFunc {
    Callable { index: usize, stack: Option<usize> },
    Captured(usize),
}
