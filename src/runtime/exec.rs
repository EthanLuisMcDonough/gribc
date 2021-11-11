use ast::node::*;
use location::Located;
use runtime::memory::*;
use runtime::operator::*;
use runtime::values::*;
use std::collections::HashMap;

fn scope_imports<'a>(
    scope: &mut Scope<'a>,
    stack: &mut Stack,
    program: &'a Program,
    import: &'a Import,
) {
    let imports = import
        .module
        .iter(program)
        .zip(import.module.names(program));

    match &import.kind {
        ImportKind::All => {
            for (callable, name) in imports {
                scope.declare_stack(stack, name, callable);
            }
        }
        ImportKind::List(hash) => {
            for (callable, name) in imports.filter(|(_, key)| hash.contains_key(*key)) {
                scope.declare_stack(stack, name, callable);
            }
        }
        ImportKind::ModuleObject(name) => scope.declare_stack(
            stack,
            &name.data,
            GribValue::ModuleObject(import.module.clone()),
        ),
    }
}

pub fn execute(program: &Program, config: GcConfig) {
    let mut stack = Stack::new();
    let mut gc = Gc::new(config);
    let mut scope = Scope::new();

    for import in &program.imports {
        scope_imports(&mut scope, &mut stack, program, import);
    }

    for (index, fnc) in program.functions.iter().enumerate() {
        scope.declare_stack(
            &mut stack,
            &fnc.identifier.data,
            Callable::Procedure {
                module: None,
                index,
            },
        );
    }

    run_block(&program.body, &mut scope, &mut stack, &mut gc, program);
}

enum ControlFlow {
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

fn run_block(
    block: &Block,
    scope: &mut Scope,
    stack: &mut Stack,
    gc: &mut Gc,
    program: &Program,
) -> ControlFlow {
    for node in block.iter() {
        match node {
            Node::Block(block) => {
                run_block(block, scope, stack, gc, program);
            }
            Node::Break => return ControlFlow::Break,
            Node::Continue => return ControlFlow::Continue,
            Node::Return(expr) => {
                return ControlFlow::Return(evaluate_expression(expr, scope, stack, gc, program))
            }
            //Node::Declaration(decl) => decl.
            _ => unimplemented!(),
        }
    }

    ControlFlow::default()
}

//fn hash_create_prop(hash: &mut Hash, )

fn bind_value(val: &mut GribValue, bind_target: usize) {
    if let GribValue::Callable(Callable::Lambda { binding, .. }) = val {
        *binding = bind_target;
    }
}

fn evaluate_hash(
    hash: &Hash,
    mutable: bool,
    scope: &mut Scope,
    stack: &mut Stack,
    gc: &mut Gc,
    program: &Program,
) -> GribValue {
    let ptr = gc.alloc_heap(HeapValue::Hash(HashValue {
        mutable,
        values: HashMap::new(),
    }));
    let mut values = HashMap::new();

    for (label, val) in hash.iter() {
        values.insert(
            label.to_string(),
            match val {
                ObjectValue::Expression(e) => {
                    let mut evaluated = evaluate_expression(e, scope, stack, gc, program);
                    bind_value(&mut evaluated, ptr);
                    evaluated.into()
                }
                ObjectValue::AutoProp(prop) => {
                    let mut set = None;

                    let get = prop.get.as_ref().and_then(|p| match p {
                        AutoPropValue::String(s) => scope
                            .capture_var(stack, gc, s.data.as_str())
                            .map(AccessFunc::Captured),
                        AutoPropValue::Lambda(ind) => AccessFunc::Callable {
                            index: *ind,
                            stack: gc.capture_stack(stack, scope, &program.getters[*ind].capture),
                            binding: ptr,
                        }
                        .into(),
                    });

                    HashPropertyValue::AutoProp { get, set }
                }
            },
        );
    }

    if let Some(HeapValue::Hash(hash)) = gc.heap_val_mut(ptr) {
        hash.values = values;
    }

    GribValue::HeapValue(ptr)
}

fn evaluate_lambda(
    body: &LambdaBody,
    mut scope: Scope,
    binding: usize,
    stack: &mut Stack,
    gc: &mut Gc,
    program: &Program,
) -> GribValue {
    scope.set_this(binding);

    let res = match body {
        LambdaBody::Block(block) => run_block(block, &mut scope, stack, gc, program).into(),
        LambdaBody::ImplicitReturn(expr) => {
            evaluate_expression(&expr, &mut scope, stack, gc, program)
        }
    };

    scope.cleanup(stack);

    res
}

// Evaluating getters
fn evaluate_access_func_get(
    fnc: &AccessFunc,
    scope: &mut Scope,
    stack: &mut Stack,
    gc: &mut Gc,
    program: &Program,
) -> GribValue {
    match fnc {
        AccessFunc::Callable {
            stack: stack_index,
            index: getter_index,
            binding,
        } => {
            let mut new_scope = scope.clone();

            if let Some(captures) = gc.get_captured_stack(*stack_index) {
                for (key, index) in captures {
                    new_scope.add_existing_captured(stack, key, *index);
                }
            }

            evaluate_lambda(
                &program.getters[*getter_index].block,
                new_scope,
                *binding,
                stack,
                unimplemented!(),
                program,
            )
        }
        AccessFunc::Captured(captured) => gc.normalize_val(*captured),
    }
}

fn property_access(
    expression: &Expression,
    key: &String,
    scope: &mut Scope,
    stack: &mut Stack,
    gc: &mut Gc,
    program: &Program,
) -> GribValue {
    let value = evaluate_expression(&*expression, scope, stack, gc, program);

    match value.ptr().and_then(|ind| gc.heap_val(ind)) {
        Some(HeapValue::Hash(HashValue { values, .. })) => {
            match values.get(key) {
                Some(HashPropertyValue::AutoProp {
                    get: Some(getter), ..
                }) => {
                    //evaluate_access_func_get(getter, scope, gc, program)
                    unimplemented!()
                }
                Some(HashPropertyValue::Value(val)) => val.clone(),
                _ => GribValue::Nil,
            }
        }
        _ => GribValue::Nil,
    }
}

pub fn evaluate_expression(
    expression: &Expression,
    scope: &mut Scope,
    stack: &mut Stack,
    gc: &mut Gc,
    program: &Program,
) -> GribValue {
    match expression {
        Expression::Bool(b) => GribValue::Bool(*b),
        Expression::Nil => GribValue::Nil,
        Expression::This => scope.get_this(gc),
        Expression::Number(f) => GribValue::Number(*f),
        Expression::String(s) => gc.alloc_str(s.clone()),
        Expression::Hash(h) => evaluate_hash(h, false, scope, stack, gc, program),
        Expression::MutableHash(h) => evaluate_hash(h, true, scope, stack, gc, program),
        Expression::ArrayCreation(expressions) => {
            let mut array = vec![];
            for e in expressions {
                array.push(evaluate_expression(e, scope, stack, gc, program));
            }
            GribValue::HeapValue(gc.alloc_heap(HeapValue::Array(array)))
        }
        Expression::Identifier(Located { data, .. }) => {
            scope.get(stack, gc, data).cloned().unwrap_or_default()
        }
        Expression::PropertyAccess { item, property } => {
            property_access(&*item, property, scope, stack, gc, program)
        }
        _ => unimplemented!(),
    }
}
