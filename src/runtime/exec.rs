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

pub fn execute(program: &Program, config: GcConfig) {
    let mut stack = Stack::new();
    let mut gc = Gc::new(config);
    let mut scope = Scope::new();

    for import in &program.imports {
        scope_imports(&mut scope, &mut stack, program, import);
    }

    for (index, fnc) in program.functions.iter().enumerate() {
        scope.declare_stack(
            &mut stack,
            fnc.identifier.data,
            Callable::Procedure {
                module: None,
                index,
            },
        );
    }

    run_block(&program.body, &mut scope, &mut stack, program, &mut gc);
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

fn declare(
    decl: &Declaration,
    scope: &mut Scope,
    stack: &mut Stack,
    program: &Program,
    gc: &mut Gc,
) {
    for declaration in &decl.declarations {
        let value = evaluate_expression(&declaration.value, scope, stack, program, gc);
        let label = declaration.identifier.data;
        if declaration.captured {
            scope.declare_stack(stack, label, value);
        } else {
            scope.declare_captured(stack, gc, label, value);
        }
    }
}

fn run_block(
    block: &Block,
    scope: &mut Scope,
    stack: &mut Stack,
    program: &Program,
    gc: &mut Gc,
) -> Option<ControlFlow> {
    for node in block.iter() {
        match node {
            Node::Block(block) => {
                control_guard!(run_block(block, scope, stack, program, gc));
            }
            Node::Break => return Some(ControlFlow::Break),
            Node::Continue => return Some(ControlFlow::Continue),
            Node::Return(expr) => {
                return ControlFlow::Return(evaluate_expression(expr, scope, stack, program, gc))
                    .into()
            }
            Node::Declaration(decl) => declare(decl, scope, stack, program, gc),
            Node::Expression(expression) => {
                evaluate_expression(expression, scope, stack, program, gc);
            }
            Node::LogicChain {
                if_block,
                elseifs,
                else_block,
            } => {
                let first_cond =
                    evaluate_expression(&if_block.condition, scope, stack, program, gc);
                if truthy(&first_cond, program, gc) {
                    control_guard!(run_block(&if_block.block, scope, stack, program, gc));
                } else {
                    let mut run_else = true;
                    for ConditionBodyPair { condition, block } in elseifs {
                        let cond = evaluate_expression(&condition, scope, stack, program, gc);
                        if truthy(&cond, program, gc) {
                            run_else = false;
                            control_guard!(run_block(&block, scope, stack, program, gc));
                            break;
                        }
                    }

                    if let Some(block) = else_block.filter(|_| run_else) {
                        control_guard!(run_block(&block, scope, stack, program, gc));
                    }
                }
            }
            Node::While(pair) => {
                while {
                    let cond = evaluate_expression(&pair.condition, scope, stack, program, gc);
                    truthy(&cond, program, gc)
                } {
                    let val = run_block(&pair.block, scope, stack, program, gc);
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
                    declare(d, scope, stack, program, gc);
                }

                while condition
                    .map(|c| evaluate_expression(&c, scope, stack, program, gc))
                    .map(|g| truthy(&g, program, gc))
                    .unwrap_or(true)
                {
                    control_guard!(run_block(body, scope, stack, program, gc));

                    if let Some(incr_expr) = increment {
                        evaluate_expression(incr_expr, scope, stack, program, gc);
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
    stack: &mut Stack,
    program: &Program,
    gc: &mut Gc,
) -> GribValue {
    let ptr = gc.alloc_heap(HeapValue::Hash(HashValue {
        mutable,
        values: HashMap::new(),
    }));
    let mut values = HashMap::new();

    for (label, val) in hash.iter() {
        values.insert(
            label.to_string(),
            match val {
                ObjectValue::Expression(e) => {
                    let mut evaluated = evaluate_expression(e, scope, stack, program, gc);
                    bind_value(&mut evaluated, ptr);
                    evaluated.into()
                }
                ObjectValue::AutoProp(prop) => {
                    let get = prop.get.as_ref().and_then(|p| match p {
                        AutoPropValue::String(s) => scope
                            .capture_var(stack, gc, s.data)
                            .map(AccessFunc::Captured),
                        AutoPropValue::Lambda(ind) => AccessFunc::Callable {
                            index: *ind,
                            stack: gc.capture_stack(stack, scope, &program.getters[*ind].capture),
                        }
                        .into(),
                    });

                    let set = prop.set.as_ref().and_then(|p| match p {
                        AutoPropValue::String(s) => scope
                            .capture_var(stack, gc, s.data)
                            .map(AccessFunc::Captured),
                        AutoPropValue::Lambda(ind) => AccessFunc::Callable {
                            index: *ind,
                            stack: gc.capture_stack(stack, scope, &program.setters[*ind].capture),
                        }
                        .into(),
                    });

                    HashPropertyValue::AutoProp { get, set }
                }
            },
        );
    }

    if let Some(HeapValue::Hash(hash)) = gc.heap_val_mut(ptr) {
        unimplemented!()
    }

    GribValue::HeapValue(ptr)
}

pub fn evaluate_lambda(
    body: &LambdaBody,
    mut scope: Scope,
    binding: Option<usize>,
    stack: &mut Stack,
    program: &Program,
    gc: &mut Gc,
) -> GribValue {
    if let Some(i) = binding {
        scope.set_this(i);
    }

    let res = match body {
        LambdaBody::Block(block) => match run_block(block, &mut scope, stack, program, gc) {
            Some(ControlFlow::Return(val)) => val,
            _ => GribValue::Nil,
        },
        LambdaBody::ImplicitReturn(expr) => {
            evaluate_expression(&expr, &mut scope, stack, program, gc)
        }
    };

    scope.cleanup(stack);

    res
}

// Evaluating getters
fn evaluate_access_func_get(
    fnc: &AccessFunc,
    scope: &mut Scope,
    stack: &mut Stack,
    program: &Program,
    gc: &mut Gc,
    binding: usize,
) -> GribValue {
    match fnc {
        AccessFunc::Callable {
            stack: stack_index,
            index: getter_index,
        } => {
            let mut new_scope = scope.clone();

            if let Some(captures) = stack_index.and_then(|i| gc.get_captured_stack(i)) {
                for (key, index) in captures {
                    new_scope.add_existing_captured(stack, *key, *index);
                }
            }

            evaluate_lambda(
                &program.getters[*getter_index].block,
                new_scope,
                binding.into(),
                stack,
                program,
                gc,
            )
        }
        AccessFunc::Captured(captured) => gc.normalize_val(*captured),
    }
}

fn property_access(
    expression: &Expression,
    key: &String,
    scope: &mut Scope,
    stack: &mut Stack,
    program: &Program,
    gc: &mut Gc,
) -> GribValue {
    let value = evaluate_expression(&*expression, scope, stack, program, gc);

    match value.ptr().and_then(|ind| gc.heap_val(ind)) {
        Some(HeapValue::Hash(hash_value)) => {
            hash_value.get_property(value.as_str(program: &'a Program, gc: &'a Gc))
        }
        _ => GribValue::Nil,
    }
}

pub fn evaluate_expression(
    expression: &Expression,
    scope: &mut Scope,
    stack: &mut Stack,
    program: &Program,
    gc: &mut Gc,
) -> GribValue {
    match expression {
        Expression::Bool(b) => GribValue::Bool(*b),
        Expression::Nil => GribValue::Nil,
        Expression::This => scope.get_this(gc),
        Expression::Number(f) => GribValue::Number(*f),
        Expression::String(s) => gc.alloc_str(program.strings[*s].clone()),
        Expression::Hash(h) => evaluate_hash(h, false, scope, stack, program, gc),
        Expression::MutableHash(h) => evaluate_hash(h, true, scope, stack, program, gc),
        Expression::ArrayCreation(expressions) => {
            let mut array = vec![];
            for e in expressions {
                array.push(evaluate_expression(e, scope, stack, program, gc));
            }
            GribValue::HeapValue(gc.alloc_heap(HeapValue::Array(array)))
        }
        Expression::Identifier(Located { data, .. }) => {
            scope.get(stack, gc, *data).cloned().unwrap_or_default()
        }
        Expression::PropertyAccess { item, property } => property_access(
            &*item,
            &program.strings[*property],
            scope,
            stack,
            program,
            gc,
        ),
        Expression::IndexAccess { item, index } => {
            let item = evaluate_expression(item.as_ref(), scope, stack, program, gc);
            let index = evaluate_expression(index.as_ref(), scope, stack, program, gc);
            index_access(item, index, scope, stack, program, gc)
        }
        _ => unimplemented!(),
    }
}
