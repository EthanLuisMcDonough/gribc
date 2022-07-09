use super::LiveAssignable;
use ast::node::Program;
use operators::{Assignment, Binary, Unary};
use runtime::{exec::*, memory::*, values::*};

fn add_values(
    left: &GribValue,
    right: &GribValue,
    program: &Program,
    runtime: &mut Runtime,
) -> GribValue {
    if let Some(arr) = runtime.gc.try_get_array_mut(left.clone()) {
        let mut new_arr = arr.clone();
        new_arr.push(right.clone());
        GribValue::HeapValue(runtime.alloc_heap(HeapValue::Array(new_arr)))
    } else if left.is_string() || right.is_string() {
        let mut new_str = left.as_str(program, runtime).into_owned();
        new_str.push_str(right.as_str(program, runtime).as_ref());
        runtime.alloc_str(new_str).into()
    } else {
        GribValue::Number(
            left.cast_num(program, &runtime.gc) + right.cast_num(program, &runtime.gc),
        )
    }
}

fn sub_values(left: &GribValue, right: &GribValue, program: &Program, gc: &Gc) -> GribValue {
    GribValue::Number(left.cast_num(program, gc) - right.cast_num(program, gc))
}

fn mult_values(
    left: &GribValue,
    right: &GribValue,
    program: &Program,
    runtime: &mut Runtime,
) -> GribValue {
    if let Some(arr) = runtime.gc.try_get_array(left.clone()) {
        let mut new_arr = Vec::new();

        if let Some(range) = right.cast_ind(program, &runtime.gc) {
            new_arr.reserve(arr.len() * range);
            for _ in 0..range {
                for value in arr.iter() {
                    new_arr.push(value.clone());
                }
            }
        }

        GribValue::HeapValue(runtime.alloc_heap(HeapValue::Array(new_arr)))
    } else {
        if let Some(KnownIndex::String(ptr)) = left.ptr().and_then(|i| runtime.gc.typed_index(i)) {
            ptr.get(&runtime.gc)
                .and_then(|s| right.cast_ind(program, &runtime.gc).map(|i| s.repeat(i)))
                .map(|s| runtime.alloc_str(s))
                .unwrap_or_default()
                .into()
        } else {
            GribValue::Number(
                left.cast_num(program, &runtime.gc) * right.cast_num(program, &runtime.gc),
            )
        }
    }
}

fn div_values(left: &GribValue, right: &GribValue, program: &Program, gc: &Gc) -> GribValue {
    GribValue::Number(left.cast_num(program, gc) / right.cast_num(program, gc))
}

fn mod_values(left: &GribValue, right: &GribValue, program: &Program, gc: &Gc) -> GribValue {
    GribValue::Number(left.cast_num(program, gc) % right.cast_num(program, gc))
}

pub fn binary_expr(
    op: &Binary,
    left: &GribValue,
    right: &Expression,
    this: &GribValue,
    runtime: &mut Runtime,
    program: &Program,
) -> GribValue {
    use self::Binary::*;
    if op.is_lazy() {
        GribValue::Bool(if let &LogicalAnd = op {
            left.truthy(program, &runtime.gc)
                && evaluate_expression(right, this, runtime, program).truthy(program, &runtime.gc)
        } else {
            // LogicalOr
            left.truthy(program, &runtime.gc)
                || evaluate_expression(right, this, runtime, program).truthy(program, &runtime.gc)
        })
    } else {
        let right_expr = evaluate_expression(right, this, runtime, program);
        match op {
            Plus => add_values(left, &right_expr, program, runtime),
            Minus => sub_values(left, &right_expr, program, &runtime.gc),
            Mult => mult_values(left, &right_expr, program, runtime),
            Div => div_values(left, &right_expr, program, &runtime.gc),
            Mod => mod_values(left, &right_expr, program, &runtime.gc),
            LogicalAnd | LogicalOr => panic!("Unreachable arm"),
            Equal | NotEqual => GribValue::Bool(
                left.exact_equals(&right_expr, program, &runtime.gc) == (op == &Equal),
            ),
            GreaterThan | LessEq | LessThan | GreaterEq => left
                .coerced_cmp(&right_expr, program, runtime)
                .map(|c| match op {
                    LessThan => c.is_lt(),
                    GreaterThan => c.is_gt(),
                    LessEq => c.is_le(),
                    GreaterEq => c.is_ge(),
                    _ => panic!("Unreachable arm"),
                })
                .unwrap_or(false)
                .into(),
        }
    }
}

pub fn unary_expr(op: &Unary, val: &GribValue, gc: &Gc, program: &Program) -> GribValue {
    match op {
        Unary::LogicalNegation => (!val.truthy(program, gc)).into(),
        Unary::Negation => (-val.cast_num(program, gc)).into(),
    }
}

pub fn assignment_expr(
    op: &Assignment,
    left: &Assignable,
    right: GribValue,
    this: &GribValue,
    runtime: &mut Runtime,
    program: &Program,
) -> GribValue {
    LiveAssignable::new(left, this, runtime, program)
        .map(|live| {
            let mut val = || live.get(runtime, program);
            let res = match op {
                Assignment::Assign => right.clone(),
                Assignment::AssignDiv => div_values(&val(), &right, program, &runtime.gc),
                Assignment::AssignMinus => sub_values(&val(), &right, program, &runtime.gc),
                Assignment::AssignMod => mod_values(&val(), &right, program, &runtime.gc),
                Assignment::AssignMult => {
                    let val = val();

                    // Check for array
                    // If so, repeat array R times
                    right
                        .cast_ind(program, &runtime.gc)
                        .and_then(|right| {
                            runtime.gc.try_get_array_mut(val.clone()).map(|array| {
                                if right == 0 {
                                    array.clear();
                                    return val.clone();
                                }

                                let r = right * array.len();
                                let count = right.checked_sub(1).unwrap_or(0);

                                array.reserve(r);
                                for _ in 0..count {
                                    let mut copy = array.clone();
                                    array.append(&mut copy);
                                }

                                val.clone()
                            })
                        })
                        .unwrap_or_else(|| mult_values(&val, &right, program, runtime))
                }
                Assignment::AssignPlus => {
                    let val = val();

                    // Check for array
                    // If so, repeat array R times
                    runtime
                        .gc
                        .try_get_array_mut(val.clone())
                        .map(|array| {
                            array.push(right.clone());
                            val.clone()
                        })
                        .unwrap_or_else(|| add_values(&val, &right, program, runtime))
                }
            };

            live.set(runtime, program, res)
        })
        .unwrap_or(right)
}
