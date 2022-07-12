use super::{AccessFunc, Callable, GribString, GribValue};
use ast::node::Program;
use runtime::exec::evaluate_lambda;
use runtime::memory::{Gc, Runtime};
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
                .map(|f| match f {
                    AccessFunc::Static(val) => val.clone(),
                    AccessFunc::Captured(ptr) => runtime.gc.get_captured(*ptr).clone(),
                    AccessFunc::Callable { index, stack } => {
                        runtime
                            .stack
                            .add_call(GribValue::HeapValue(self_ptr), stack.clone());
                        let res = evaluate_lambda(&program.getters[*index].block, runtime, program);
                        runtime.stack.pop_call();
                        res
                    }
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
            *runtime.gc.get_captured_mut(*ptr) = val.clone();
            val
        }
        AccessFunc::Callable { index, stack } => {
            let setter = &program.setters[*index];
            if setter.param_captured {
                runtime.add_stack_captured(val);
            } else {
                runtime.stack.add(val);
            }

            runtime
                .stack
                .add_call(GribValue::HeapValue(self_ptr), stack.clone());
            let res = evaluate_lambda(&setter.block, runtime, program);

            runtime.stack.pop_call();
            runtime.stack.pop();

            res
        }
        AccessFunc::Static(_) => panic!("Setters cannot be static values"),
    }
}
