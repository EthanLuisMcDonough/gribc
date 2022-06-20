use super::slot::*;
use super::*;
use runtime::values::{AccessFunc, Callable, GribValue, HashPropertyValue, HeapValue};

pub fn mark_heap(gc: &mut Gc, ind: usize) {
    let Markable { marked, ref value } = &mut gc.heap[ind];

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
                for (_, val) in stack.iter() {
                    match val {
                        StackSlot::Captured(index) => marked_heap.push(*index),
                        StackSlot::Value(val) => marked_stack.push(val.clone()),
                        StackSlot::Empty => {}
                    }
                }
            }
            HeapValue::Hash(hash) => {
                for (key, value) in hash.iter() {
                    marked_stack.push(key.clone().into());
                    match value {
                        HashPropertyValue::Value(val) => marked_stack.push(val.clone()),
                        HashPropertyValue::AutoProp { get, set } => {
                            if let Some(f1) = get {
                                marked_func.push(f1.clone());
                            }
                            if let Some(f2) = set {
                                marked_func.push(f2.clone());
                            }
                        }
                    }
                }
            }
        },
        _ => to_mark = false,
    }

    *marked = to_mark;

    for value in marked_stack {
        mark_stack(gc, &value);
    }

    for value in marked_heap {
        mark_heap(gc, value);
    }

    for value in marked_func {
        match value {
            AccessFunc::Captured(ind) => mark_heap(gc, ind),
            AccessFunc::Callable { stack, .. } => {
                if let Some(ind) = stack {
                    mark_heap(gc, ind);
                }
            }
            AccessFunc::Static(val) => mark_stack(gc, &val),
        }
    }
}

pub fn mark_stack(gc: &mut Gc, obj: &GribValue) {
    match obj {
        GribValue::HeapValue(heap) | GribValue::String(GribString::Heap(heap)) => {
            mark_heap(gc, *heap)
        }
        GribValue::Callable(callable) => mark_function(gc, callable),
        _ => {}
    }
}

fn mark_function(gc: &mut Gc, fnc: &Callable) {
    match fnc {
        Callable::Lambda { binding, stack, .. } => {
            if let Some(ind) = binding {
                mark_heap(gc, *ind);
            }
            if let Some(ind) = stack {
                mark_heap(gc, *ind);
            }
        }
        _ => {}
    }
}
