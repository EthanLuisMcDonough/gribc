use ast::node::*;
use location::Located;
use runtime::memory::*;
use runtime::operator::*;
use runtime::values::*;
use std::collections::HashMap;

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

    run_block(&program.body, &mut scope, &mut runtime, program);
}

enum ControlFlow {
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
    ($control:expr) => {
        if $control.is_some() {
            return $control;
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
    scope: &mut Scope,
    runtime: &mut Runtime,
    program: &Program,
) -> Option<ControlFlow> {
    for node in block.iter() {
        match node {
            Node::Block(block) => {
                control_guard!(run_block(block, scope, runtime, program));
            }
            Node::Break => return Some(ControlFlow::Break),
            Node::Continue => return Some(ControlFlow::Continue),
            Node::Return(expr) => {
                return ControlFlow::Return(evaluate_expression(expr, scope, runtime, program))
                    .into()
            }
            Node::Declaration(decl) => declare(decl, scope, runtime, program),
            Node::Expression(expression) => {
                evaluate_expression(expression, scope, runtime, program);
            }
            Node::LogicChain {
                if_block,
                elseifs,
                else_block,
            } => {
                let first_cond = evaluate_expression(&if_block.condition, scope, runtime, program);
                if truthy(&first_cond, program, &runtime.gc) {
                    control_guard!(run_block(&if_block.block, scope, runtime, program));
                } else {
                    let mut run_else = true;
                    for ConditionBodyPair { condition, block } in elseifs {
                        let cond = evaluate_expression(&condition, scope, runtime, program);
                        if truthy(&cond, program, &runtime.gc) {
                            run_else = false;
                            control_guard!(run_block(&block, scope, runtime, program));
                            break;
                        }
                    }

                    if let Some(block) = else_block.filter(|_| run_else) {
                        control_guard!(run_block(&block, scope, runtime, program));
                    }
                }
            }
            Node::While(pair) => {
                while {
                    let cond = evaluate_expression(&pair.condition, scope, runtime, program);
                    truthy(&cond, program, &runtime.gc)
                } {
                    let val = run_block(&pair.block, scope, runtime, program);
                    match &val {
                        Some(ControlFlow::Return(_)) => return val,
                        Some(ControlFlow::Break) => break,
                        _ => {}
                    };
                }
            }
            Node::For {
                declaration,
                condition,
                increment,
                body,
            } => {
                if let Some(d) = declaration {
                    declare(d, scope, runtime, program);
                }

                while condition
                    .map(|c| evaluate_expression(&c, scope, runtime, program))
                    .map(|g| truthy(&g, program, &runtime.gc))
                    .unwrap_or(true)
                {
                    control_guard!(run_block(body, scope, runtime, program));

                    if let Some(incr_expr) = increment {
                        evaluate_expression(incr_expr, scope, runtime, program);
                    }
                }
            }
        }
    }

    None
}

//fn hash_create_prop(hash: &mut Hash, )
///@TODO remove if necessary unimplemented!
fn bind_value(val: &mut GribValue, bind_target: usize) {
    if let GribValue::Callable(Callable::Lambda { binding, .. }) = val {
        *binding = Some(bind_target);
    }
}

fn evaluate_hash(
    hash: &Hash,
    mutable: bool,
    scope: &mut Scope,
    runtime: &mut Runtime,
    program: &Program,
) -> GribValue {
    let ptr = runtime.alloc_heap(HeapValue::Hash(HashValue {
        mutable,
        values: HashMap::new(),
    }));
    let mut values = HashMap::new();

    for (label, val) in hash.iter() {
        values.insert(
            label.to_string(),
            match val {
                ObjectValue::Expression(e) => {
                    let mut evaluated = evaluate_expression(e, scope, runtime, program);
                    bind_value(&mut evaluated, ptr);
                    evaluated.into()
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
        );
    }

    if let Some(HeapValue::Hash(hash)) = runtime.gc.heap_val_mut(ptr) {
        unimplemented!()
    }

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
        LambdaBody::Block(block) => match run_block(block, &mut scope, runtime, program) {
            Some(ControlFlow::Return(val)) => val,
            _ => GribValue::Nil,
        },
        LambdaBody::ImplicitReturn(expr) => {
            evaluate_expression(&expr, &mut scope, runtime, program)
        }
    };

    scope.cleanup(&mut runtime.stack);

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

            if let Some(captures) = stack_index.and_then(|i| runtime.gc.get_captured_stack(i)) {
                for (key, index) in captures {
                    new_scope.add_existing_captured(&mut runtime.stack, *key, *index);
                }
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
