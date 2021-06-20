use ast::node::*;
use runtime::memory::*;
use runtime::values::*;

pub fn execute(program: &Program, config: GcConfig) {
    let mut gc = Gc::new(config);
    let mut scope = Scope::new();

    for import in &program.imports {
        match import.kind {
            ImportKind::All => {
                //program.modules[]
            }
            _ => unimplemented!(),
        }
    }

    for proc in &program.functions {
        //scope.set_constant(proc.identifier.data, GribValue::)
    }
}

fn try_get_array_copy<'a, 'b>(gc: &'a Gc<'b>, val: GribValue) -> Option<Vec<GribValue>> {
    if let Some(HeapValue::Array(arr)) = gc.heap_val(val) {
        Some(arr.clone())
    } else { None }
}

fn add_values<'a>(
    left: GribValue, right: GribValue, 
    scope: &mut Scope, gc: &'a mut Gc<'a>
) -> GribValue {
    let mut array_op = None;

    {
        array_op = try_get_array_copy(gc, left);
    }
    let mut arr = array_op.unwrap();
    arr.push(right);
    gc.alloc_heap(HeapValue::Array(arr));
    /*if let Some(mut arr) = array_op {
        arr.push(right);
        GribValue::HeapValue(gc.alloc_heap(HeapValue::Array(arr)))
    } else {
        unimplemented!()
    }*/
    unimplemented!()
    /*match left {
        GribValue::HeapValue(i)
        GribValue::Number(_) | GribValue::Nil | GribValue::
    }*/
}

fn evaluate_expression() {}