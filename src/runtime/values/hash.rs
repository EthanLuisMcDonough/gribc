use super::{AccessFunc, GribString, GribValue};
use ast::node::Program;
use runtime::memory::Gc;
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

    /// Gets the calculated gribvalue given by the provided key
    /// These values are not "raw"
    pub fn get_property(&self, string: &GribString, program: &Program, gc: &mut Gc) {
        let mut hasher = self.get_hasher();

        GribKey::new(string, hasher, program, gc).
    }
}
