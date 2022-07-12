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
    run_block(&program.body, &mut runtime, program);
}

#[derive(Debug)]
pub enum ControlFlow {
    Return(GribValue),
    Break,
    Continue,
}

impl ControlFlow {
    pub fn new(node: &FlowBreak, runtime: &mut Runtime, program: &Program) -> Self {
        match &node.kind {
            BreakType::Break => ControlFlow::Break,
            BreakType::Continue => ControlFlow::Continue,
            BreakType::Return(e) => ControlFlow::Return(evaluate_expression(e, runtime, program)),
        }
    }
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

fn declare(decl: &Declaration, runtime: &mut Runtime, program: &Program) {
    for declaration in &decl.declarations {
        let value = evaluate_expression(&declaration.value, runtime, program);
        if declaration.captured {
            runtime.add_stack_captured(value);
        } else {
            runtime.stack.add(value);
        }
    }
}

pub fn run_block(block: &Block, runtime: &mut Runtime, program: &Program) -> Option<ControlFlow> {
    let mut result = None;
    for node in block.iter() {
        match &node {
            Node::Block(block) => {
                control_guard!(result, run_block(block, runtime, program));
            }
            Node::ControlFlow(flow) => {
                let ret = ControlFlow::new(flow, runtime, program);
                runtime.stack.pop_stack(flow.allocations);
                return_break!(result, ret);
            }
            Node::Declaration(decl) => declare(decl, runtime, program),
            Node::Expression(expression) => {
                evaluate_expression(expression, runtime, program);
            }
            Node::LogicChain {
                if_block,
                elseifs,
                else_block,
            } => {
                let first_cond = evaluate_expression(&if_block.condition, runtime, program);
                if first_cond.truthy(program, &runtime.gc) {
                    let res = run_block(&if_block.block, runtime, program);
                    control_guard!(result, res);
                } else {
                    let mut run_else = true;
                    for ConditionBodyPair { condition, block } in elseifs {
                        let cond = evaluate_expression(&condition, runtime, program);
                        if cond.truthy(program, &runtime.gc) {
                            run_else = false;
                            control_guard!(result, run_block(&block, runtime, program));
                            break;
                        }
                    }

                    if let Some(block) = else_block.as_ref().filter(|_| run_else) {
                        control_guard!(result, run_block(&block, runtime, program));
                    }
                }
            }
            Node::While(pair) => {
                let mut local_result = None;

                while evaluate_expression(&pair.condition, runtime, program)
                    .truthy(program, &runtime.gc)
                {
                    let val = run_block(&pair.block, runtime, program);
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
                let mut params = 0;
                if let Some(d) = declaration {
                    declare(d, runtime, program);
                    params = d.declarations.len();
                }

                let mut local_result = None;
                while condition
                    .as_ref()
                    .map(|c| evaluate_expression(&c, runtime, program))
                    .map(|g| g.truthy(program, &runtime.gc))
                    .unwrap_or(true)
                {
                    let flow = run_block(body, runtime, program);
                    check_flow!(local_result, flow);

                    if let Some(incr_expr) = increment {
                        evaluate_expression(incr_expr, runtime, program);
                    }
                }

                runtime.stack.pop_stack(params);
                control_guard!(result, local_result);
            }
        }
    }

    // Don't pop block allocations if we've already popped them off
    // while evaluating the control flow
    if result.is_none() {
        runtime.stack.pop_stack(block.allocations);
    }

    result
}

fn convert_slot(slot: StackSlot) -> AccessFunc {
    match slot {
        StackSlot::Captured(ind) => AccessFunc::Captured(ind).into(),
        StackSlot::Value(val) => AccessFunc::Static(val).into(),
        StackSlot::Empty => panic!("ATTEMPT TO CONVERT EMPTY SLOT"),
    }
}

fn evaluate_hash(
    hash: &Hash,
    mutable: bool,
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
                ObjectValue::Expression(e) => evaluate_expression(e, runtime, program).into(),
                ObjectValue::AutoProp(prop) => {
                    let get = prop.get.as_ref().map(|p| match p {
                        AutoPropValue::String(_s) => {
                            panic!("STRING AUTOPROP SHOULD NOT BE FOUND DURING RUNTIME")
                        }
                        AutoPropValue::Value(RuntimeValue::Static(static_val)) => {
                            AccessFunc::Static(static_val.clone().into())
                        }
                        AutoPropValue::Value(RuntimeValue::Stack(ptr)) => {
                            convert_slot(runtime.read_slot(*ptr).clone())
                        }
                        AutoPropValue::Lambda(ind) => AccessFunc::Callable {
                            index: *ind,
                            stack: runtime.capture_stack(&program.getters[*ind].capture),
                        },
                    });

                    let set = prop.set.as_ref().map(|p| match p {
                        AutoPropValue::Value(RuntimeValue::Static(_))
                        | AutoPropValue::String(_) => panic!("Invalid setter found during runtime"),
                        AutoPropValue::Value(RuntimeValue::Stack(ptr)) => {
                            if let StackSlot::Captured(ind) = runtime.read_slot(*ptr) {
                                AccessFunc::Captured(*ind)
                            } else {
                                panic!("FAILED TO CAPTURE ACCESS SETTER SLOT {:?}", ptr);
                            }
                        }
                        AutoPropValue::Lambda(ind) => AccessFunc::Callable {
                            index: *ind,
                            stack: runtime.capture_stack(&program.setters[*ind].capture),
                        },
                    });

                    HashPropertyValue::AutoProp { get, set }
                }
            },
        )
    }

    runtime.gc.set_heap_val_at(HeapValue::Hash(values), ptr);

    GribValue::HeapValue(ptr)
}

pub fn evaluate_lambda(body: &LambdaBody, runtime: &mut Runtime, program: &Program) -> GribValue {
    let res = match body {
        LambdaBody::Block(block) => match run_block(block, runtime, program) {
            Some(ControlFlow::Return(value)) => value,
            _ => GribValue::Nil,
        },
        LambdaBody::ImplicitReturn(expr) => evaluate_expression(&expr, runtime, program),
    };

    res
}

fn eval_list(items: &Vec<Expression>, runtime: &mut Runtime, program: &Program) -> Vec<GribValue> {
    items
        .iter()
        .map(|e| evaluate_expression(e, runtime, program))
        .collect()
}

pub fn evaluate_expression(
    expression: &Expression,
    runtime: &mut Runtime,
    program: &Program,
) -> GribValue {
    use self::Expression::*;
    match expression {
        Bool(b) => GribValue::Bool(*b),
        Nil => GribValue::Nil,
        This { .. } => runtime.stack.get_this(),
        Number(f) => GribValue::Number(*f),
        String(s) => runtime.alloc_str(program.strings[*s].clone()).into(),
        Hash(h) => evaluate_hash(h, false, runtime, program),
        MutableHash(h) => evaluate_hash(h, true, runtime, program),
        ArrayCreation(expressions) => {
            let array = eval_list(expressions, runtime, program);
            GribValue::HeapValue(runtime.alloc_heap(HeapValue::Array(array)))
        }
        Identifier(Located { data, .. }) => panic!(
            "Invalid identifier found during runtime ({})",
            program.strings[*data]
        ),
        PropertyAccess { item, property } => {
            let value = evaluate_expression(item.as_ref(), runtime, program);
            LiveProperty::new(value, *property, &runtime.gc, program)
                .map(|prop| prop.get(runtime, program))
                .unwrap_or_default()
        }
        IndexAccess { item, index } => {
            let item = evaluate_expression(item.as_ref(), runtime, program);
            let index = evaluate_expression(index.as_ref(), runtime, program);
            LiveIndex::new(item, &index, runtime, program)
                .map(|ind| ind.get(runtime, program))
                .unwrap_or_default()
        }
        Unary { op, expr } => {
            let val = evaluate_expression(expr, runtime, program);
            unary_expr(op, &val, &runtime.gc, program)
        }
        Binary { op, left, right } => {
            let left_val = evaluate_expression(left, runtime, program);
            binary_expr(op, &left_val, right.as_ref(), runtime, program)
        }
        Assignment { op, left, right } => {
            let val = evaluate_expression(right, runtime, program);
            assignment_expr(op, left, val, runtime, program)
        }
        FunctionCall { function, args } => {
            let values = eval_list(args, runtime, program);
            let fn_val = evaluate_expression(function, runtime, program);
            if let GribValue::Callable(f) = fn_val {
                f.call(program, runtime, values)
            } else {
                GribValue::Nil
            }
        }
        Lambda(index) => GribValue::Callable(Callable::Lambda {
            binding: None,
            stack: runtime.capture_stack(&program.lambdas[*index].capture),
            index: *index,
        }),
        Value(val) => match val {
            RuntimeValue::Stack(ptr) => runtime.read_val(*ptr).clone(),
            RuntimeValue::Static(static_val) => static_val.clone().into(),
        },
    }
}
