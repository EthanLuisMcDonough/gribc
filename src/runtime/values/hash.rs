use super::{AccessFunc, Callable, GribString, GribValue};
use ast::node::Program;
use runtime::exec::evaluate_lambda;
use runtime::memory::{Gc, Runtime, Scope};
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash, Hasher};

#[derive(Clone, Debug)]
pub enum HashPropertyValue {
    AutoProp {
        get: Option<AccessFunc>,
        set: Option<AccessFunc>,
    },
    Value(GribValue),
}

impl From<GribValue> for HashPropertyValue {
    fn from(prop: GribValue) -> Self {
        HashPropertyValue::Value(prop)
    }
}

impl HashPropertyValue {
    pub fn get(&self, runtime: &mut Runtime, program: &Program, self_ptr: usize) -> GribValue {
        match self {
            HashPropertyValue::Value(val) => {
                let mut grib_val = val.clone();
                if let GribValue::Callable(Callable::Lambda { binding, .. }) = &mut grib_val {
                    *binding = binding.or(Some(self_ptr));
                }
                grib_val
            }
            HashPropertyValue::AutoProp { get, .. } => get
                .as_ref()
                .and_then(|f| match f {
                    AccessFunc::Captured(ptr) => runtime.gc.get_captured(*ptr).cloned(),
                    AccessFunc::Callable {
                        index,
                        stack: captured_ind,
                    } => program.getters.get(*index).and_then(|getter| {
                        let mut scope = Scope::new();
                        if let Some(i) = captured_ind {
                            scope.add_captured_stack(runtime, *i);
                        }
                        evaluate_lambda(&getter.block, scope, self_ptr.into(), runtime, program)
                            .into()
                    }),
                })
                .unwrap_or_default(),
        }
    }
}

#[derive(Clone, Eq, Debug)]
pub struct GribKey {
    hash: u64,
    string: GribString,
}

impl GribKey {
    fn new(string: GribString, mut hasher: impl Hasher, program: &Program, gc: &Gc) -> Self {
        let r = string.as_ref(program, gc).unwrap_or_default();
        GribKey {
            hash: {
                r.hash(&mut hasher);
                hasher.finish()
            },
            string,
        }
    }
}

impl Hash for GribKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl PartialEq for GribKey {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

#[derive(Clone, Debug)]
pub struct HashValue {
    mutable: bool,
    values: HashMap<GribKey, HashPropertyValue>,
}

impl HashValue {
    pub fn new(mutable: bool) -> Self {
        Self {
            mutable,
            values: HashMap::new(),
        }
    }

    pub fn key(&self, string: GribString, program: &Program, gc: &Gc) -> GribKey {
        let hasher = self.get_hasher();
        GribKey::new(string, hasher, program, gc)
    }

    /// Sets the grib hash's raw value
    /// Getters and setters can be assigned values
    pub fn init_value(&mut self, key: GribKey, value: impl Into<HashPropertyValue>) {
        self.values.insert(key, value.into());
    }

    fn get_hasher(&self) -> impl Hasher {
        self.values.hasher().build_hasher()
    }

    pub fn custom_module(module_index: usize, program: &Program, gc: &Gc) -> Self {
        let mut hash = Self::new(false);
        let module = &program.modules[module_index];

        for (index, function) in module.functions.iter().enumerate() {
            let key = hash.key(GribString::Stored(function.identifier.data), program, gc);
            hash.init_value(
                key,
                GribValue::Callable(Callable::Procedure {
                    module: module_index.into(),
                    index,
                }),
            );
        }

        hash
    }

    pub fn freeze(&mut self) {
        self.mutable = false;
    }

    pub fn is_mutable(&self) -> bool {
        self.mutable
    }

    pub fn get_property(&'_ self, key: &GribKey) -> Option<&'_ HashPropertyValue> {
        self.values.get(key)
    }

    pub fn try_set(&mut self, key: &GribKey, val: GribValue) -> Option<AccessFunc> {
        use self::HashPropertyValue::*;
        let mutable = self.mutable;

        if self.is_mutable() && !self.values.contains_key(key) {
            self.init_value(key.clone(), val);
            None
        } else {
            match self.values.get_mut(key) {
                Some(Value(r)) if mutable => {
                    *r = val;
                    None
                }
                Some(AutoProp { set, .. }) => set.clone(),
                _ => None,
            }
        }
    }

    pub fn delete_key(&mut self, key: &GribKey) {
        self.values.remove(key);
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a GribString, &'a HashPropertyValue)> {
        self.values
            .iter()
            .map(|(raw_key, val)| (&raw_key.string, val))
    }

    pub fn keys(&self) -> Vec<GribValue> {
        self.values
            .keys()
            .map(|key| key.string.clone())
            .map(GribValue::String)
            .collect()
    }

    pub fn into_values(
        self,
        runtime: &mut Runtime,
        program: &Program,
        self_ptr: usize,
    ) -> Vec<(String, GribValue)> {
        self.values
            .into_iter()
            .flat_map(|(raw_key, raw_value)| {
                raw_key
                    .string
                    .as_ref(program, &runtime.gc)
                    .map(|r| r.to_string())
                    .map(|s| (s, raw_value.get(runtime, program, self_ptr)))
            })
            .collect()
    }
}

pub fn eval_setter(
    func: &AccessFunc,
    runtime: &mut Runtime,
    program: &Program,
    self_ptr: usize,
    val: GribValue,
) -> GribValue {
    match func {
        AccessFunc::Captured(ptr) => {
            if let Some(r) = runtime.gc.get_captured_mut(*ptr) {
                *r = val.clone();
            }
            val
        }
        AccessFunc::Callable { index, stack } => {
            let setter = &program.setters[*index];

            let mut scope = Scope::new();
            if let Some(i) = stack {
                scope.add_captured_stack(runtime, *i);
            }
            scope.declare_stack(&mut runtime.stack, setter.param, val);
            evaluate_lambda(&setter.block, scope, self_ptr.into(), runtime, program)
        }
    }
}
