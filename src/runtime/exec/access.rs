/// Structures related to getting and setting index and property values
use super::evaluate_expression;
use ast::node::{Assignable, Program};
use runtime::{
    memory::{Gc, Runtime, Scope},
    native_fn::NativePackage,
    values::{eval_setter, Callable, GribKey, GribString, GribValue, KnownIndex},
};

pub enum LiveProperty {
    Hash { key: GribKey, ptr: usize },
    Module { key: usize, module: NativePackage },
}

impl LiveProperty {
    pub fn new(value: GribValue, key: usize, gc: &Gc, program: &Program) -> Option<Self> {
        if let GribValue::ModuleObject(module) = value {
            Some(Self::Module { module, key })
        } else {
            value.ptr().and_then(|ptr| {
                gc.try_get_hash(ptr).map(|hash| {
                    let str_val = GribString::Stored(key);
                    let key = hash.key(str_val, program, gc);
                    Self::Hash { ptr, key }
                })
            })
        }
    }

    pub fn get(&self, runtime: &mut Runtime, program: &Program) -> GribValue {
        match &self {
            Self::Hash { key, ptr } => runtime
                .gc
                .try_get_hash(*ptr)
                .and_then(|hash| hash.get_property(key).cloned())
                .map(|prop| prop.get(runtime, program, *ptr)),
            Self::Module { key, module } => module
                .fn_from_str(&program.strings[*key])
                .map(Callable::Native)
                .map(GribValue::Callable),
        }
        .unwrap_or_default()
    }

    pub fn set(&self, runtime: &mut Runtime, program: &Program, val: GribValue) -> GribValue {
        match &self {
            Self::Hash { key, ptr } => runtime
                .gc
                .try_get_hash_mut(*ptr)
                .and_then(|hash| hash.try_set(key, val.clone()))
                .map(|setter| eval_setter(&setter, runtime, program, *ptr, val.clone()))
                .unwrap_or(val),
            Self::Module { .. } => val,
        }
    }
}

pub enum LiveIndex {
    Hash {
        ptr: usize,
        index: GribKey,
    },
    Array {
        ptr: usize,
        index: usize,
    },
    String {
        string: GribString,
        index: usize,
    },
    Module {
        module: NativePackage,
        index: GribString,
    },
}

impl LiveIndex {
    pub fn new(
        item: GribValue,
        index: &GribValue,
        runtime: &mut Runtime,
        program: &Program,
    ) -> Option<Self> {
        match item {
            GribValue::ModuleObject(module) => LiveIndex::Module {
                module: module,
                index: index.to_str(runtime),
            }
            .into(),
            GribValue::String(string) => index
                .cast_ind(program, &runtime.gc)
                .map(|index| LiveIndex::String { index, string }),
            GribValue::HeapValue(ptr) => runtime.gc.typed_index(ptr).and_then(|v| match v {
                KnownIndex::Hash(hash_ref) => {
                    let str_val = index.to_str(runtime);
                    hash_ref
                        .get(&runtime.gc)
                        .map(|hash| hash.key(str_val, program, &runtime.gc))
                        .map(|index| LiveIndex::Hash { ptr, index })
                }
                KnownIndex::Array(_) => index
                    .cast_ind(program, &runtime.gc)
                    .map(|index| LiveIndex::Array { ptr, index }),
                KnownIndex::String(_) => None,
            }),
            _ => None,
        }
    }

    pub fn get(&self, runtime: &mut Runtime, program: &Program) -> GribValue {
        match &self {
            Self::Hash { ptr, index } => runtime
                .gc
                .try_get_hash(*ptr)
                .and_then(|hash| hash.get_property(index).cloned())
                .map(|prop| prop.get(runtime, program, *ptr)),
            Self::Array { ptr, index } => runtime
                .gc
                .try_get_array(*ptr)
                .and_then(|arr| arr.get(*index).cloned()),
            Self::String { string, index } => string
                .as_ref(program, &runtime.gc)
                .and_then(|r| r.char_at(*index))
                .map(GribString::Char)
                .map(GribValue::String),
            Self::Module { module, index } => index
                .as_ref(program, &runtime.gc)
                .and_then(|r| module.fn_from_str(r.borrow().as_ref()))
                .map(Callable::Native)
                .map(GribValue::Callable),
        }
        .unwrap_or_default()
    }

    pub fn set(&self, runtime: &mut Runtime, program: &Program, val: GribValue) -> GribValue {
        match &self {
            Self::Hash { ptr, index } => runtime
                .gc
                .try_get_hash_mut(*ptr)
                .and_then(|hash| hash.try_set(index, val.clone()))
                .map(|setter| eval_setter(&setter, runtime, program, *ptr, val))
                .unwrap_or_default(),
            Self::Array { ptr, index } => {
                if let Some(slot) = runtime
                    .gc
                    .try_get_array_mut(*ptr)
                    .and_then(|arr| arr.get_mut(*index))
                {
                    *slot = val.clone()
                }
                val
            }
            Self::String { .. } | Self::Module { .. } => val,
        }
    }
}

pub enum LiveAssignable {
    Identifier(usize),
    Index(LiveIndex),
    Property(LiveProperty),
}

impl LiveAssignable {
    pub fn new(
        assignable: &Assignable,
        scope: &mut Scope,
        runtime: &mut Runtime,
        program: &Program,
    ) -> Option<Self> {
        match assignable {
            Assignable::Identifier(ident) => Self::Identifier(ident.data).into(),
            Assignable::IndexAccess { item, index } => {
                let item_val = evaluate_expression(item, scope, runtime, program);
                let index_val = evaluate_expression(index, scope, runtime, program);
                LiveIndex::new(item_val, &index_val, runtime, program).map(LiveAssignable::Index)
            }
            Assignable::PropertyAccess { item, property } => {
                let item_val = evaluate_expression(item, scope, runtime, program);
                LiveProperty::new(item_val, *property, &runtime.gc, program)
                    .map(LiveAssignable::Property)
            }
        }
    }

    pub fn get(&self, scope: &mut Scope, runtime: &mut Runtime, program: &Program) -> GribValue {
        match self {
            Self::Identifier(ident) => scope.get(runtime, *ident).cloned().unwrap_or_default(),
            Self::Index(index) => index.get(runtime, program),
            Self::Property(property) => property.get(runtime, program),
        }
    }

    pub fn set(
        &self,
        scope: &mut Scope,
        runtime: &mut Runtime,
        program: &Program,
        val: GribValue,
    ) -> GribValue {
        match self {
            Self::Identifier(ident) => {
                scope.set(runtime, *ident, val.clone());
                val
            }
            Self::Index(index) => index.set(runtime, program, val),
            Self::Property(prop) => prop.set(runtime, program, val),
        }
    }
}
