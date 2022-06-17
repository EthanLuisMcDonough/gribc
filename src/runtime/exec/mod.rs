mod access;
mod operator;

use self::access::*;
use self::operator::*;

use ast::node::*;
use location::Located;
use runtime::memory::*;
use runtime::values::*;

pub fn execute(program: &Program, config: RuntimeConfig) {
    let mut runtime = Runtime::new(config);
    let mut scope = Scope::new();

    scope.scope_imports(&mut runtime, program, &program.imports);
    scope.scope_functions(&mut runtime, &program.functions, None);

    runtime.base_scope = scope;

    run_block(
        &program.body,
        runtime.base_scope.clone(),
        &mut runtime,
        program,
    );
}

#[derive(Debug)]
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
    ($name:ident, $control:expr) => {{
        let _t = $control;
        if _t.is_some() {
            $name = _t;
            break;
        }
    }};
}
macro_rules! return_break {
    ($name:ident, $control:expr) => {{
        $name = $control.into();
        break;
    }};
}
macro_rules! check_flow {
    ($name:ident, $control:expr) => {{
        let _t = $control;
        match &_t {
            Some(ControlFlow::Return(_)) => return_break!($name, _t),
            Some(ControlFlow::Break) => {
                break;
            }
            Some(ControlFlow::Continue) | None => {}
        }
    }};
}

fn declare(decl: &Declaration, scope: &mut Scope, runtime: &mut Runtime, program: &Program) {
    for declaration in &decl.declarations {
        let value = evaluate_expression(&declaration.value, scope, runtime, program);
        let label = declaration.identifier.data;
        if declaration.captured {
            scope.declare_captured(runtime, label, value);
        } else {
            scope.declare_stack(&mut runtime.stack, label, value);
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
            Node::Break => {
                return_break!(result, ControlFlow::Break)
            }
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
                if first_cond.truthy(program, &runtime.gc) {
                    let res = run_block(&if_block.block, scope.clone(), runtime, program);
                    control_guard!(result, res);
                } else {
                    let mut run_else = true;
                    for ConditionBodyPair { condition, block } in elseifs {
                        let cond = evaluate_expression(&condition, &mut scope, runtime, program);
                        if cond.truthy(program, &runtime.gc) {
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

                while evaluate_expression(&pair.condition, &mut scope, runtime, program)
                    .truthy(program, &runtime.gc)
                {
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
                    .map(|g| g.truthy(program, &runtime.gc))
                    .unwrap_or(true)
                {
                    let flow = run_block(body, scope.clone(), runtime, program);
                    check_flow!(local_result, flow);

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
        let key = values.key(GribString::Stored(*label), program, &runtime.gc);
        values.init_value(
            key,
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

fn eval_list(
    items: &Vec<Expression>,
    scope: &mut Scope,
    runtime: &mut Runtime,
    program: &Program,
) -> Vec<GribValue> {
    items
        .iter()
        .map(|e| evaluate_expression(e, scope, runtime, program))
        .collect()
}

pub fn evaluate_expression(
    expression: &Expression,
    scope: &mut Scope,
    runtime: &mut Runtime,
    program: &Program,
) -> GribValue {
    use self::Expression::*;
    match expression {
        Bool(b) => GribValue::Bool(*b),
        Nil => GribValue::Nil,
        This => scope.get_this(&runtime.gc),
        Number(f) => GribValue::Number(*f),
        String(s) => runtime.alloc_str(program.strings[*s].clone()).into(),
        Hash(h) => evaluate_hash(h, false, scope, runtime, program),
        MutableHash(h) => evaluate_hash(h, true, scope, runtime, program),
        ArrayCreation(expressions) => {
            let array = eval_list(expressions, scope, runtime, program);
            GribValue::HeapValue(runtime.alloc_heap(HeapValue::Array(array)))
        }
        Identifier(Located { data, .. }) => scope.get(runtime, *data).cloned().unwrap_or_default(),
        PropertyAccess { item, property } => {
            let value = evaluate_expression(item.as_ref(), scope, runtime, program);
            LiveProperty::new(value, *property, &runtime.gc, program)
                .map(|prop| prop.get(runtime, program))
                .unwrap_or_default()
        }
        IndexAccess { item, index } => {
            let item = evaluate_expression(item.as_ref(), scope, runtime, program);
            let index = evaluate_expression(index.as_ref(), scope, runtime, program);
            LiveIndex::new(item, &index, runtime, program)
                .map(|ind| ind.get(runtime, program))
                .unwrap_or_default()
        }
        Unary { op, expr } => {
            let val = evaluate_expression(expr, scope, runtime, program);
            unary_expr(op, &val, &runtime.gc, program)
        }
        Binary { op, left, right } => {
            let left_val = evaluate_expression(left, scope, runtime, program);
            binary_expr(op, &left_val, right.as_ref(), scope, runtime, program)
        }
        Assignment { op, left, right } => {
            let val = evaluate_expression(right, scope, runtime, program);
            assignment_expr(op, left, val, scope, runtime, program)
        }
        FunctionCall { function, args } => {
            let values = eval_list(args, scope, runtime, program);
            let fn_val = evaluate_expression(function, scope, runtime, program);
            if let GribValue::Callable(f) = fn_val {
                f.call(program, runtime, values)
            } else {
                GribValue::Nil
            }
        }
        Lambda(index) => GribValue::Callable(Callable::Lambda {
            binding: None,
            stack: runtime.capture_stack(scope, &program.lambdas[*index].captured),
            index: *index,
        }),
    }
}
