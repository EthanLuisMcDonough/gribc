use ast::node::Parameters;
use ast::node::{Import, Procedure, Program};
use runtime::memory::{slot::*, Gc, Runtime, Stack};
use runtime::values::{Callable, GribValue, HashValue, HeapValue};
use std::collections::HashMap;

const STACK_OVERFLOW_MSG: &str = "Grib stack overflow";

#[derive(Debug)]
pub struct Scope {
    scope: HashMap<usize, usize>,
    local_count: usize,
    this: GribValue,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            scope: HashMap::new(),
            local_count: 0,
            this: GribValue::Nil,
        }
    }

    pub fn get_this(&self, gc: &Gc) -> GribValue {
        gc.normalize_val(self.this.clone())
    }

    pub fn set_this(&mut self, this: impl Into<GribValue>) {
        self.this = this.into();
    }

    fn declare(&mut self, label: usize, ptr: usize) {
        self.scope.insert(label, ptr);
        self.local_count += 1;
    }

    pub fn declare_stack(&mut self, stack: &mut Stack, label: usize, value: impl Into<GribValue>) {
        let ptr = stack
            .add(StackSlot::Value(value.into()))
            .expect(STACK_OVERFLOW_MSG);
        self.declare(label, ptr);
    }

    pub fn declare_heap(&mut self, runtime: &mut Runtime, label: usize, value: HeapValue) {
        let heap_ptr = runtime.alloc_heap(value);
        let val = StackSlot::Value(GribValue::HeapValue(heap_ptr));
        let ptr = runtime.stack.add(val).expect(STACK_OVERFLOW_MSG);
        self.declare(label, ptr);
    }

    pub fn declare_captured(&mut self, runtime: &mut Runtime, label: usize, value: GribValue) {
        let heap_ptr = runtime.alloc_captured(value);
        let val = StackSlot::Captured(heap_ptr);
        let ptr = runtime.stack.add(val).expect(STACK_OVERFLOW_MSG);
        self.declare(label, ptr);
    }

    pub fn cleanup(self, stack: &mut Stack) {
        stack.pop_stack(self.local_count);
    }

    pub fn get_slot<'a>(&self, stack: &'a Stack, label: usize) -> Option<&'a StackSlot> {
        self.scope.get(&label).and_then(|ind| stack.stack.get(*ind))
    }

    pub fn get<'a>(&self, runtime: &'a Runtime, label: usize) -> Option<&'a GribValue> {
        self.scope
            .get(&label)
            .cloned()
            .and_then(|slot| runtime.get_stack(slot))
    }

    fn get_mut<'a>(&self, runtime: &'a mut Runtime, label: usize) -> Option<&'a mut GribValue> {
        self.scope
            .get(&label)
            .cloned()
            .and_then(move |slot| runtime.get_stack_mut(slot))
    }

    pub fn set(&self, runtime: &mut Runtime, label: usize, value: GribValue) {
        if let Some(r) = self.get_mut(runtime, label) {
            *r = value;
        }
    }

    pub fn add_captured_stack(&mut self, runtime: &mut Runtime, ptr: usize) {
        if let Some(HeapValue::CapturedStack(stack_ref)) = runtime.gc.heap_val(ptr) {
            for (label, value) in stack_ref {
                let ptr = runtime.stack.add(value.clone()).expect(STACK_OVERFLOW_MSG);
                self.declare(*label, ptr);
            }
        }
    }

    pub fn add_params(&mut self, params: &Parameters, runtime: &mut Runtime, args: Vec<GribValue>) {
        let mut arg_iter = args.into_iter();

        for param in &params.params {
            let label = param.name;
            let val = arg_iter.next().unwrap_or_default();
            if param.captured {
                self.declare_captured(runtime, label, val);
            } else {
                self.declare_stack(&mut runtime.stack, label, val);
            }
        }

        if let Some(spread) = &params.vardic {
            let args = HeapValue::Array(arg_iter.collect());
            let name = spread.name;
            if spread.captured {
                let ptr = runtime.alloc_heap(args);
                self.declare_captured(runtime, name, GribValue::HeapValue(ptr));
            } else {
                self.declare_heap(runtime, name, args);
            }
        }
    }

    pub fn scope_imports(
        &mut self,
        runtime: &mut Runtime,
        program: &Program,
        imports: &Vec<Import>,
    ) {
        use ast::node::{ImportKind, Module};

        for import in imports {
            let imports = import.module.callables(program).into_iter();

            match &import.kind {
                ImportKind::All => {
                    for (callable, name) in imports {
                        self.declare_stack(&mut runtime.stack, name, callable);
                    }
                }
                ImportKind::List(hash) => {
                    for (callable, name) in imports.filter(|(_, key)| hash.contains_key(key)) {
                        self.declare_stack(&mut runtime.stack, name, callable);
                    }
                }
                ImportKind::ModuleObject(name) => match &import.module {
                    Module::Custom(ind) => {
                        let hash = HashValue::custom_module(*ind, program, &runtime.gc);
                        self.declare_heap(runtime, name.data, HeapValue::Hash(hash));
                    }
                    Module::Native { package, .. } => self.declare_stack(
                        &mut runtime.stack,
                        name.data,
                        GribValue::ModuleObject(package.clone()),
                    ),
                },
            }
        }
    }

    pub fn scope_functions(
        &mut self,
        runtime: &mut Runtime,
        functions: &Vec<Procedure>,
        module: Option<usize>,
    ) {
        for (index, fnc) in functions.iter().enumerate() {
            self.declare_stack(
                &mut runtime.stack,
                fnc.identifier.data,
                Callable::Procedure { module, index },
            );
        }
    }
}

impl Clone for Scope {
    fn clone(&self) -> Self {
        Self {
            local_count: 0,
            scope: self.scope.clone(),
            this: self.this.clone(),
        }
    }
}
