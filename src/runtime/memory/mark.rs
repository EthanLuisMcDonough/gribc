use super::slot::*;
use super::*;
use runtime::values::{AccessFunc, Callable, GribValue, HeapValue};

pub fn mark(runtime: &mut Runtime, ind: usize) {
    let Markable { marked, ref value } = &mut runtime.gc.heap[ind];

    if *marked {
        return;
    }

    let mut to_mark = true;
    let mut marked_stack = Vec::new();
    let mut marked_heap = Vec::new();
    let mut marked_func = Vec::new();

    match value {
        HeapSlot::Captured(val) => marked_stack.push(val.clone()), // shouldn't recurse deeper than one level
        HeapSlot::Value(val) => match val {
            HeapValue::String(_) => {}
            HeapValue::Array(arr) => {
                for i in arr.iter() {
                    marked_stack.push(i.clone());
                }
            }
            HeapValue::CapturedStack(stack) => {
                for (_, index) in stack.iter() {
                    marked_heap.push(*index);
                }
            }
            HeapValue::Hash(hash) => {
                unimplemented!()
                /*for (_, value) in &hash.values {
                    match value {
                        HashPropertyValue::Value(val) => marked_stack.push(val.clone()),
                        /*HashPropertyValue::AutoProp(prop) => {
                            for f in prop.functions() {
                                marked_func.push(f.clone());
                            }
                        }*/
                        _ => unimplemented!(),
                    }
                }*/
            }
        },
        _ => to_mark = false,
    }

    *marked = to_mark;

    for value in marked_stack {
        mark_stack(runtime, value);
    }

    for value in marked_heap {
        mark(runtime, value);
    }

    for value in marked_func {
        match value {
            AccessFunc::Captured(ind) => mark(runtime, ind),
            AccessFunc::Callable { stack, .. } => {
                if let Some(ind) = stack {
                    mark(runtime, ind);
                }
            }
        }
    }
}

fn mark_stack(runtime: &mut Runtime, obj: GribValue) {
    match obj {
        GribValue::HeapValue(heap) => mark(runtime, heap),
        GribValue::Callable(callable) => mark_function(runtime, callable),
        _ => {}
    }
}

fn mark_function(runtime: &mut Runtime, fnc: Callable) {
    match fnc {
        Callable::Lambda { binding, stack, .. } => {
            if let Some(ind) = binding {
                mark(runtime, ind);
            }
            if let Some(ind) = stack {
                mark(runtime, ind);
            }
        }
        _ => {}
    }
}
