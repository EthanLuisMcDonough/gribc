use ast::node::expression::*;
use ast::node::Program;
use operators::{Assignment, Binary, Unary};
use runtime::exec::*;
use runtime::memory::*;
use runtime::values::*;

fn add_values(left: &GribValue, right: &GribValue, program: &Program, gc: &mut Gc) -> GribValue {
    if let Some(arr) = gc.try_get_array_mut(left.clone()) {
        let mut new_arr = arr.clone();
        new_arr.push(right.clone());
        GribValue::HeapValue(gc.alloc_heap(HeapValue::Array(new_arr)))
    } else {
        if let Some(string) = gc.try_get_string(left, program) {
            let mut new_str = string.to_string();
            new_str.push_str(right.as_str(program, gc).as_ref());
            gc.alloc_str(new_str).into()
        } else {
            GribValue::Number(left.cast_num(program, gc) + right.cast_num(program, gc))
        }
    }
}

fn sub_values(left: &GribValue, right: &GribValue, program: &Program, gc: &Gc) -> GribValue {
    GribValue::Number(left.cast_num(program, gc) - right.cast_num(program, gc))
}

fn mult_values(left: &GribValue, right: &GribValue, program: &Program, gc: &mut Gc) -> GribValue {
    if let Some(arr) = gc.try_get_array(left.clone()) {
        let mut new_arr = Vec::new();

        if let Some(range) = right.cast_ind(program, gc) {
            for _ in 0..range {
                for value in arr.iter() {
                    new_arr.push(value.clone());
                }
            }
        }

        GribValue::HeapValue(gc.alloc_heap(HeapValue::Array(new_arr)))
    } else {
        if let Some(string) = gc.try_get_string(left, program) {
            gc.alloc_str(
                right
                    .cast_ind(program, gc)
                    .map(|i| string.repeat(i))
                    .unwrap_or_default(),
            )
            .into()
        } else {
            GribValue::Number(left.cast_num(program, gc) + right.cast_num(program, gc))
        }
    }
}

fn div_values(left: &GribValue, right: &GribValue, program: &Program, gc: &Gc) -> GribValue {
    GribValue::Number(left.cast_num(program, gc) / right.cast_num(program, gc))
}

fn mod_values(left: &GribValue, right: &GribValue, program: &Program, gc: &Gc) -> GribValue {
    GribValue::Number(left.cast_num(program, gc) % right.cast_num(program, gc))
}

pub fn index_access(
    item: GribValue,
    index: GribValue,
    scope: &mut Scope,
    stack: &mut Stack,
    program: &Program,
    gc: &mut Gc,
) -> GribValue {
    match item {
        GribValue::String(s) => match s.as_ref(program, gc).expect("String not found") {
            GribStringRef::Ref(r) => index
                .cast_ind(program, gc)
                .and_then(|i| r.chars().nth(i))
                .map(GribString::Char)
                .map(GribValue::String)
                .unwrap_or_default(),
            GribStringRef::Char(c) => index
                .cast_ind(program, gc)
                .filter(|&i| i == 0)
                .map(|_| c)
                .map(GribString::Char)
                .map(GribValue::String)
                .unwrap_or_default(),
        },
        GribValue::HeapValue(s) => match item.ptr().and_then(|ptr| gc.heap_val(ptr)) {
            Some(HeapValue::Array(arr)) => index
                .cast_ind(program, gc)
                .and_then(|i| arr.get(i).cloned())
                .map(|val| gc.normalize_val(val))
                .unwrap_or_default(),
            _ => GribValue::Nil,
        },
        _ => GribValue::Nil,
    }
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
            truthy(left, program, gc)
                && truthy(
                    &evaluate_expression(right, scope, stack, program, gc),
                    program,
                    gc,
                )
        } else {
            // LogicalOr
            truthy(left, program, gc)
                || truthy(
                    &evaluate_expression(right, scope, stack, program, gc),
                    program,
                    gc,
                )
        })
    } else {
        let right_expr = evaluate_expression(right, scope, stack, program, gc);
        match op {
            Binary::Plus => add_values(left, &right_expr, program, gc),
            Binary::Minus => sub_values(left, &right_expr, program, gc),
            Binary::Mult => mult_values(left, &right_expr, program, gc),
            Binary::Div => div_values(left, &right_expr, program, gc),
            Binary::Mod => mod_values(left, &right_expr, program, gc),

            _ => unimplemented!(),
            Binary::LogicalAnd | Binary::LogicalOr => panic!("Unreachable arm"),
        }
    }
}

pub fn truthy(value: &GribValue, program: &Program, gc: &Gc) -> bool {
    match value {
        GribValue::Callable(_) | GribValue::ModuleObject(_) => true,
        GribValue::Number(n) => *n != 0.0,
        GribValue::Nil => false,
        GribValue::String(s) => s.as_ref(program, gc).map(|s| !s.is_empty()).unwrap_or(true),
        GribValue::HeapValue(heap) => unimplemented!(),
        GribValue::Bool(b) => *b,
    }
}
