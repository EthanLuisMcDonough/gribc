use ast::node::expression::*;
use operators::{Assignment, Binary, Unary};
use runtime::exec::*;
use runtime::memory::*;
use runtime::values::*;

fn try_get_array<'a>(gc: &'a Gc, val: GribValue) -> Option<&'a Vec<GribValue>> {
    if let Some(HeapValue::Array(arr)) = gc.heap_val(val) {
        Some(arr)
    } else {
        None
    }
}

fn try_get_array_mut<'a>(gc: &'a mut Gc, val: GribValue) -> Option<&'a mut Vec<GribValue>> {
    if let Some(HeapValue::Array(arr)) = gc.heap_val_mut(val) {
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
        if let Some(string) = gc.get_str(left.clone()) {
            let mut new_str = string.clone();
            new_str.push_str(right.as_str(gc).as_ref());
            gc.alloc_str(new_str)
        } else {
            GribValue::Number(left.cast_num(gc) + right.cast_num(gc))
        }
    }
}

fn sub_values(left: &GribValue, right: &GribValue, gc: &Gc) -> GribValue {
    GribValue::Number(left.cast_num(gc) * right.cast_num(gc))
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
        if let Some(string) = gc.get_str(left.clone()) {
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

pub fn binary_expr(op: &Binary, left: &GribValue, right: &Expression, gc: &mut Gc) -> GribValue {
    match op {
        /*Binary::Plus => add_values(left, evaluate_expression(right, gc), gc),
        Binary::Minus => sub_values(left, evaluate_expression(right), gc),
        Binary::Mult => mult_values(left, evaluate_expression(right), gc),
        Binary::Div => div_values(left, evaluate_expression(right), gc),*/
        //Binary::LogicalAnd => truthy(left, gc) && truthy(right, gc),
        //Binary::LogicalOr => truthy(left, gc) || truthy(right, gc),
        _ => unimplemented!(),
    }
}

//pub fn truthy(value: GribValue, gc: )
