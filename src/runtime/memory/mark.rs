use super::slot::*;
use super::*;
use runtime::values::{AccessFunc, Callable, GribValue, HashPropertyValue, HeapValue};

pub fn mark_heap(gc: &mut Gc, ind: usize) {
    use self::{HeapValue::*, MemSlot::*};

    let Markable { marked, ref value } = &mut gc.heap[ind];

    // Don't bother with marked/empty slots
    if let Some(value) = value.as_ref().filter(|_| !*marked) {
        let mut marked_stack = Vec::new();
        let mut marked_heap = Vec::new();
        let mut marked_func = Vec::new();

        match value {
            Captured(val) => marked_stack.push(val.clone()),
            Value(Array(arr)) => {
                for i in arr.iter() {
                    marked_stack.push(i.clone());
                }
            }
            Value(CapturedStack(stack)) => {
                for val in stack {
                    match val {
                        StackSlot::Captured(index) => marked_heap.push(*index),
                        StackSlot::Value(val) => marked_stack.push(val.clone()),
                    }
                }
            }
            Value(Hash(hash)) => {
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
            Value(String(_)) => {}
        }

        *marked = true;

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
