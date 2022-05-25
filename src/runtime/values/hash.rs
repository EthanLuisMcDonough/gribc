use super::{AccessFunc, Callable, GribString, GribValue};
use ast::node::Program;
use runtime::exec::evaluate_lambda;
use runtime::memory::{Gc, Runtime, Scope};
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash, Hasher};

#[derive(Clone)]
pub enum HashPropertyValue {
    AutoProp {
        get: Option<AccessFunc>,
        set: Option<AccessFunc>,
    },
    Value(GribValue),
}

#[derive(Clone, PartialEq, Eq)]
struct GribKey {
    hash: u64,
    string: GribString,
}

impl GribKey {
    fn new(
        string: GribString,
        mut hasher: impl Hasher,
        program: &Program,
        gc: &Gc,
    ) -> Option<Self> {
        string.as_ref(program, gc).map(|r| GribKey {
            hash: {
                r.hash(&mut hasher);
                hasher.finish()
            },
            string,
        })
    }
}

impl Hash for GribKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

#[derive(Clone)]
pub struct HashValue {
    mutable: bool,
    values: HashMap<GribKey, HashPropertyValue>,
}

impl From<GribValue> for HashPropertyValue {
    fn from(prop: GribValue) -> Self {
        HashPropertyValue::Value(prop)
    }
}

enum RawValue {
    Prop(AccessFunc),
    Value(GribValue),
}

impl HashValue {
    /// Sets the grib hash's raw value
    /// Getters and setters can be assigned values
    pub fn init_value(
        &mut self,
        string: GribString,
        value: impl Into<HashPropertyValue>,
        program: &Program,
        gc: &Gc,
    ) {
        let mut hasher = self.get_hasher();

        if let Some(key) = GribKey::new(string, hasher, program, gc) {
            self.values.insert(key, value.into());
        }
    }

    fn get_hasher(&self) -> impl Hasher {
        self.values.hasher().build_hasher()
    }

    pub fn freeze(&mut self) {
        self.mutable = false;
    }

    pub fn is_mutable(&self) -> bool {
        self.mutable
    }

    fn get_raw_property(
        &self,
        string: GribString,
        program: &Program,
        gc: &mut Gc,
    ) -> Option<HashPropertyValue> {
        let mut hasher = self.get_hasher();
        GribKey::new(string, hasher, program, gc)
            .and_then(|key| self.values.get(&key))
            .cloned()
    }

    /// Gets the calculated gribvalue given by the provided key
    /// These values are not "raw"
    pub fn get_property(
        &self,
        string: GribString,
        runtime: &mut Runtime,
        program: &Program,
        self_ptr: usize,
    ) -> GribValue {
        self.get_raw_property(string, program, &mut runtime.gc)
            .and_then(|val| eval_raw_get_property(val, runtime, program, self_ptr))
            .unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn values(
        &self,
        runtime: &mut Runtime,
        program: &Program,
        self_ptr: usize,
    ) -> HashMap<String, GribValue> {
        let mut values = HashMap::new();

        for (raw_key, raw_value) in &self.values {
            if let Some(key) = raw_key.string.as_ref(program, &runtime.gc) {
                let val = eval_raw_get_property(raw_value.clone(), runtime, program, self_ptr);
                if let Some(v) = val {
                    values.insert(key.to_string(), v);
                }
            }
        }

        values
    }

    pub fn to_str(&self, runtime: &mut Runtime, program: &Program, self_ptr: usize) -> String {
        let mut joined = if self.is_mutable() { '$' } else { '#' }.to_string();

        joined.push('{');

        for (key, value) in self.values(runtime, program, self_ptr).into_iter() {
            joined.push_str(key.as_ref());
            joined.push_str("->");
            joined.push_str(value.as_str(program, runtime).as_ref());
        }

        if !self.is_empty() {
            joined.pop();
        }

        joined.push('}');

        joined
    }
}

fn eval_raw_get_property(
    val: HashPropertyValue,
    runtime: &mut Runtime,
    program: &Program,
    self_ptr: usize,
) -> Option<GribValue> {
    match val {
        HashPropertyValue::Value(val) => {
            let mut grib_val = val.clone();
            if let GribValue::Callable(Callable::Lambda { binding, .. }) = &mut grib_val {
                *binding = Some(self_ptr);
            }
            grib_val.into()
        }
        HashPropertyValue::AutoProp { get, .. } => get.and_then(|f| match f {
            AccessFunc::Captured(ptr) => runtime.gc.get_captured(ptr),
            AccessFunc::Callable {
                index,
                stack: captured_ind,
            } => program.getters.get(index).and_then(|getter| {
                let mut scope = Scope::new();
                if let Some(i) = captured_ind {
                    scope.add_captured_stack(runtime, i);
                }
                evaluate_lambda(&getter.block, scope, self_ptr.into(), runtime, program).into()
            }),
        }),
    }
}
