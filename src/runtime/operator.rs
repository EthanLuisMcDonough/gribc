use ast::node::expression::*;
use ast::node::Program;
use operators::{Assignment, Binary, Unary};
use runtime::exec::*;
use runtime::memory::*;
use runtime::values::*;

fn try_get_array<'a>(gc: &'a Gc, val: GribValue) -> Option<&'a Vec<GribValue>> {
    if let Some(HeapValue::Array(arr)) = val.ptr().and_then(|ptr| gc.heap_val(ptr)) {
        Some(arr)
    } else {
        None
    }
}

fn try_get_array_mut<'a>(gc: &'a mut Gc, val: GribValue) -> Option<&'a mut Vec<GribValue>> {
    if let Some(HeapValue::Array(arr)) = val.ptr().and_then(move |ptr| gc.heap_val_mut(ptr)) {
        Some(arr)
    } else {
        None
    }
}

fn add_values(left: &GribValue, right: &GribValue, gc: &mut Gc) -> GribValue {
    if let Some(arr) = try_get_array_mut(gc, left.clone()) {
        let mut new_arr = arr.clone();
        new_arr.push(right.clone());
        GribValue::HeapValue(gc.alloc_heap(HeapValue::Array(new_arr)))
    } else {
        if let Some(string) = left.ptr().and_then(|ptr| gc.get_str(ptr)) {
            let mut new_str = string.clone();
            new_str.push_str(right.as_str(gc).as_ref());
            gc.alloc_str(new_str)
        } else {
            GribValue::Number(left.cast_num(gc) + right.cast_num(gc))
        }
    }
}

fn sub_values(left: &GribValue, right: &GribValue, gc: &Gc) -> GribValue {
    GribValue::Number(left.cast_num(gc) - right.cast_num(gc))
}

fn mult_values(left: &GribValue, right: &GribValue, gc: &mut Gc) -> GribValue {
    if let Some(arr) = try_get_array(gc, left.clone()) {
        let mut new_arr = Vec::new();

        if let Some(range) = right.cast_ind(gc) {
            for _ in 0..range {
                for value in arr.iter() {
                    new_arr.push(value.clone());
                }
            }
        }

        GribValue::HeapValue(gc.alloc_heap(HeapValue::Array(new_arr)))
    } else {
        if let Some(string) = left.ptr().and_then(|ptr| gc.get_str(ptr)) {
            gc.alloc_str(
                right
                    .cast_ind(gc)
                    .map(|i| string.repeat(i))
                    .unwrap_or_default(),
            )
        } else {
            GribValue::Number(left.cast_num(gc) + right.cast_num(gc))
        }
    }
}

fn div_values(left: &GribValue, right: &GribValue, gc: &Gc) -> GribValue {
    GribValue::Number(left.cast_num(gc) / right.cast_num(gc))
}

fn mod_values(left: &GribValue, right: &GribValue, gc: &Gc) -> GribValue {
    GribValue::Number(left.cast_num(gc) % right.cast_num(gc))
}

pub fn binary_expr(
    op: &Binary,
    left: &GribValue,
    right: &Expression,
    scope: &mut Scope,
    stack: &mut Stack,
    gc: &mut Gc,
    program: &Program,
) -> GribValue {
    if op.is_lazy() {
        GribValue::Bool(if let &Binary::LogicalAnd = op {
            truthy(left, gc) && truthy(&evaluate_expression(right, scope, stack, gc, program), gc)
        } else {
            // LogicalOr
            truthy(left, gc) || truthy(&evaluate_expression(right, scope, stack, gc, program), gc)
        })
    } else {
        let right_expr = evaluate_expression(right, scope, stack, gc, program);
        match op {
            Binary::Plus => add_values(left, &right_expr, gc),
            Binary::Minus => sub_values(left, &right_expr, gc),
            Binary::Mult => mult_values(left, &right_expr, gc),
            Binary::Div => div_values(left, &right_expr, gc),
            Binary::Mod => mod_values(left, &right_expr, gc),
            _ => unimplemented!(),
            Binary::LogicalAnd | Binary::LogicalOr => panic!("Unreachable arm"),
        }
    }
}

pub fn truthy(value: &GribValue, gc: &Gc) -> bool {
    match value {
        GribValue::Callable(_) | GribValue::ModuleObject(_) => true,
        GribValue::Number(n) => *n != 0.0,
        GribValue::Nil => false,
        GribValue::HeapValue(heap) => gc.get_str(*heap).map(|s| s != "").unwrap_or(true),
        GribValue::Bool(b) => *b,
    }
}
