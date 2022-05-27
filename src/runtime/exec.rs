use ast::node::*;
use location::Located;
use runtime::memory::*;
use runtime::operator::*;
use runtime::values::*;

fn scope_imports<'a>(
    scope: &mut Scope,
    stack: &mut Stack,
    program: &'a Program,
    import: &'a Import,
) {
    let imports = import.module.callables(program).into_iter();

    match &import.kind {
        ImportKind::All => {
            for (callable, name) in imports {
                scope.declare_stack(stack, name, callable);
            }
        }
        ImportKind::List(hash) => {
            for (callable, name) in imports.filter(|(_, key)| hash.contains_key(key)) {
                scope.declare_stack(stack, name, callable);
            }
        }
        ImportKind::ModuleObject(name) => scope.declare_stack(
            stack,
            name.data,
            GribValue::ModuleObject(import.module.clone()),
        ),
    }
}

pub fn execute(program: &Program, config: RuntimeConfig) {
    let mut runtime = Runtime::new(config);
    let mut scope = Scope::new();

    for import in &program.imports {
        scope_imports(&mut scope, &mut runtime.stack, program, import);
    }

    for (index, fnc) in program.functions.iter().enumerate() {
        scope.declare_stack(
            &mut runtime.stack,
            fnc.identifier.data,
            Callable::Procedure {
                module: None,
                index,
            },
        );
    }

    run_block(&program.body, scope, &mut runtime, program);
}

pub enum ControlFlow {
    Return(GribValue),
    Break,
    Continue,
}

impl Default for ControlFlow {
    fn default() -> Self {
        Self::Return(GribValue::Nil)
    }
}

impl From<ControlFlow> for GribValue {
    fn from(f: ControlFlow) -> Self {
        if let ControlFlow::Return(val) = f {
            val
        } else {
            GribValue::Nil
        }
    }
}

macro_rules! control_guard {
    ($name:ident, $control:expr) => {
        if $control.is_some() {
            $name = $control;
            break;
        }
    };
}
macro_rules! return_break {
    ($name:ident, $control:expr) => {{
        $name = $control.into();
        break;
    }};
}
macro_rules! check_flow {
    ($name:ident, $control:expr) => {
        match &($control) {
            Some(ControlFlow::Return(_)) => return_break!($name, $control),
            Some(ControlFlow::Break) => break,
            Some(ControlFlow::Continue) | None => {}
        }
    };
}

fn declare(decl: &Declaration, scope: &mut Scope, runtime: &mut Runtime, program: &Program) {
    for declaration in &decl.declarations {
        let value = evaluate_expression(&declaration.value, scope, runtime, program);
        let label = declaration.identifier.data;
        if declaration.captured {
            scope.declare_stack(&mut runtime.stack, label, value);
        } else {
            scope.declare_captured(runtime, label, value);
        }
    }
}

pub fn run_block(
    block: &Block,
    mut scope: Scope,
    runtime: &mut Runtime,
    program: &Program,
) -> Option<ControlFlow> {
    let mut result = None;
    for node in block.iter() {
        match &node {
            Node::Block(block) => {
                control_guard!(result, run_block(block, scope.clone(), runtime, program));
            }
            Node::Break => return_break!(result, ControlFlow::Break),
            Node::Continue => return_break!(result, ControlFlow::Continue),
            Node::Return(expr) => {
                return_break!(
                    result,
                    ControlFlow::Return(evaluate_expression(expr, &mut scope, runtime, program))
                );
            }
            Node::Declaration(decl) => declare(decl, &mut scope, runtime, program),
            Node::Expression(expression) => {
                evaluate_expression(expression, &mut scope, runtime, program);
            }
            Node::LogicChain {
                if_block,
                elseifs,
                else_block,
            } => {
                let first_cond =
                    evaluate_expression(&if_block.condition, &mut scope, runtime, program);
                if truthy(&first_cond, program, &runtime.gc) {
                    control_guard!(
                        result,
                        run_block(&if_block.block, scope.clone(), runtime, program)
                    );
                } else {
                    let mut run_else = true;
                    for ConditionBodyPair { condition, block } in elseifs {
                        let cond = evaluate_expression(&condition, &mut scope, runtime, program);
                        if truthy(&cond, program, &runtime.gc) {
                            run_else = false;
                            control_guard!(
                                result,
                                run_block(&block, scope.clone(), runtime, program)
                            );
                            break;
                        }
                    }

                    if let Some(block) = else_block.as_ref().filter(|_| run_else) {
                        control_guard!(result, run_block(&block, scope.clone(), runtime, program));
                    }
                }
            }
            Node::While(pair) => {
                let mut local_result = None;

                while {
                    let cond = evaluate_expression(&pair.condition, &mut scope, runtime, program);
                    truthy(&cond, program, &runtime.gc)
                } {
                    let val = run_block(&pair.block, scope.clone(), runtime, program);
                    check_flow!(local_result, val);
                }

                control_guard!(result, local_result);
            }
            Node::For {
                declaration,
                condition,
                increment,
                body,
            } => {
                if let Some(d) = declaration {
                    declare(d, &mut scope, runtime, program);
                }

                let mut local_result = None;

                while condition
                    .as_ref()
                    .map(|c| evaluate_expression(&c, &mut scope, runtime, program))
                    .map(|g| truthy(&g, program, &runtime.gc))
                    .unwrap_or(true)
                {
                    check_flow!(
                        local_result,
                        run_block(body, scope.clone(), runtime, program)
                    );

                    if let Some(incr_expr) = increment {
                        evaluate_expression(incr_expr, &mut scope, runtime, program);
                    }
                }

                control_guard!(result, local_result);
            }
        }
    }

    scope.cleanup(&mut runtime.stack);

    result
}

fn evaluate_hash(
    hash: &Hash,
    mutable: bool,
    scope: &mut Scope,
    runtime: &mut Runtime,
    program: &Program,
) -> GribValue {
    let ptr = runtime.reserve_slot();
    let mut values = HashValue::new(mutable);

    for (label, val) in hash.iter() {
        values.init_value(
            GribString::Stored(*label),
            match val {
                ObjectValue::Expression(e) => {
                    evaluate_expression(e, scope, runtime, program).into()
                }
                ObjectValue::AutoProp(prop) => {
                    let get = prop.get.as_ref().and_then(|p| match p {
                        AutoPropValue::String(s) => {
                            scope.capture_var(runtime, s.data).map(AccessFunc::Captured)
                        }
                        AutoPropValue::Lambda(ind) => AccessFunc::Callable {
                            index: *ind,
                            stack: runtime.capture_stack(scope, &program.getters[*ind].capture),
                        }
                        .into(),
                    });

                    let set = prop.set.as_ref().and_then(|p| match p {
                        AutoPropValue::String(s) => {
                            scope.capture_var(runtime, s.data).map(AccessFunc::Captured)
                        }
                        AutoPropValue::Lambda(ind) => AccessFunc::Callable {
                            index: *ind,
                            stack: runtime.capture_stack(scope, &program.setters[*ind].capture),
                        }
                        .into(),
                    });

                    HashPropertyValue::AutoProp { get, set }
                }
            },
            program,
            &runtime.gc,
        )
    }

    runtime.gc.set_heap_val_at(HeapValue::Hash(values), ptr);

    GribValue::HeapValue(ptr)
}

pub fn evaluate_lambda(
    body: &LambdaBody,
    mut scope: Scope,
    binding: Option<usize>,
    runtime: &mut Runtime,
    program: &Program,
) -> GribValue {
    if let Some(i) = binding {
        scope.set_this(i);
    }

    let res = match body {
        LambdaBody::Block(block) => match run_block(block, scope, runtime, program) {
            Some(ControlFlow::Return(val)) => val,
            _ => GribValue::Nil,
        },
        LambdaBody::ImplicitReturn(expr) => {
            let result = evaluate_expression(&expr, &mut scope, runtime, program);
            scope.cleanup(&mut runtime.stack);
            result
        }
    };

    res
}

// Evaluating getters
fn evaluate_access_func_get(
    fnc: &AccessFunc,
    scope: &mut Scope,
    runtime: &mut Runtime,
    program: &Program,
    binding: usize,
) -> GribValue {
    match fnc {
        AccessFunc::Callable {
            stack: stack_index,
            index: getter_index,
        } => {
            let mut new_scope = scope.clone();

            if let Some(ind) = stack_index {
                new_scope.add_captured_stack(runtime, *ind);
            }

            evaluate_lambda(
                &program.getters[*getter_index].block,
                new_scope,
                binding.into(),
                runtime,
                program,
            )
        }
        AccessFunc::Captured(captured) => runtime.gc.normalize_val(*captured),
    }
}

fn property_access(
    expression: &Expression,
    key: &String,
    scope: &mut Scope,
    runtime: &mut Runtime,
    program: &Program,
) -> GribValue {
    let value = evaluate_expression(&*expression, scope, runtime, program);

    match value.ptr().and_then(|ind| runtime.gc.heap_val(ind)) {
        Some(HeapValue::Hash(hash_value)) => unimplemented!(), //hash_value.get_property(value.as_str(program, gc)),
        _ => GribValue::Nil,
    }
}

pub fn evaluate_expression(
    expression: &Expression,
    scope: &mut Scope,
    runtime: &mut Runtime,
    program: &Program,
) -> GribValue {
    match expression {
        Expression::Bool(b) => GribValue::Bool(*b),
        Expression::Nil => GribValue::Nil,
        Expression::This => scope.get_this(&runtime.gc),
        Expression::Number(f) => GribValue::Number(*f),
        Expression::String(s) => runtime.alloc_str(program.strings[*s].clone()).into(),
        Expression::Hash(h) => evaluate_hash(h, false, scope, runtime, program),
        Expression::MutableHash(h) => evaluate_hash(h, true, scope, runtime, program),
        Expression::ArrayCreation(expressions) => {
            let mut array = vec![];
            for e in expressions {
                array.push(evaluate_expression(e, scope, runtime, program));
            }
            GribValue::HeapValue(runtime.alloc_heap(HeapValue::Array(array)))
        }
        Expression::Identifier(Located { data, .. }) => {
            scope.get(runtime, *data).cloned().unwrap_or_default()
        }
        Expression::PropertyAccess { item, property } => {
            property_access(&*item, &program.strings[*property], scope, runtime, program)
        }
        Expression::IndexAccess { item, index } => {
            let item = evaluate_expression(item.as_ref(), scope, runtime, program);
            let index = evaluate_expression(index.as_ref(), scope, runtime, program);
            index_access(item, index, scope, runtime, program)
        }
        _ => unimplemented!(),
    }
}
