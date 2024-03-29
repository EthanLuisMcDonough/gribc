mod callable;
mod hash;
mod heap;
mod string;

use ast::node::*;
use runtime::memory::{Gc, Runtime};
use std::borrow::Cow;
use std::cmp::Ordering;

pub use self::callable::*;
pub use self::hash::*;
pub use self::heap::*;
pub use self::string::*;

/*pub fn float_to_ind(f: f64) -> Option<usize> {
    Some(f.trunc())
        .filter(|&i| i.is_finite() && i >= 0. && i <= (usize::MAX as f64))
        .map(|i| i as usize)
}

fn array_to_string(arr: &Vec<GribValue>, program: &Program, runtime: &mut Runtime) -> String {
    let mut joined = String::from("[");
    let empty = arr.is_empty();

    for value in arr {
        joined.push_str(value.as_str(program, runtime).as_ref());
        joined.push(',');
    }

    if empty {
        joined.pop();
    }
    joined.push(']');

    joined
}

fn values_to_string(
    v: Vec<(String, GribValue)>,
    mutable: bool,
    program: &Program,
    runtime: &mut Runtime,
) -> String {
    let mut joined = if mutable { '$' } else { '#' }.to_string();

    joined.push('{');
    let empty = v.is_empty();

    for (key, value) in v {
        joined.push_str(key.as_ref());
        joined.push_str("->");
        joined.push_str(value.as_str(program, runtime).as_ref());
    }

    if !empty {
        joined.pop();
    }

    joined.push('}');

    joined
}*/

#[derive(Clone, PartialEq, Debug)]
pub enum GribValue {
    Nil,
    Number(f64),
    String(GribString),
    Callable(Callable),
    ModuleObject(Module),
    HeapValue(usize),
    Bool(bool),
    Error(Box<GribValue>),
}

impl GribValue {
    pub fn ptr(&self) -> Option<usize> {
        match self {
            Self::HeapValue(i) | Self::String(GribString::Heap(i)) => Some(*i),
            _ => None,
        }
    }

    pub fn is_nil(&self) -> bool {
        self == &Self::Nil
    }

    fn is_number(&self) -> bool {
        if let Self::Number(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_string(&self) -> bool {
        if let Self::String(_) = self {
            true
        } else {
            false
        }
    }

    pub fn err(s: &'static str) -> Self {
        Self::Error(Self::String(GribString::Static(s)).into())
    }

    pub fn is_err(&self) -> bool {
        if let Self::Error(_) = self {
            true
        } else {
            false
        }
    }

    pub fn exact_equals(&self, val: &GribValue, program: &Program, gc: &Gc) -> bool {
        if let (Self::String(s1), Self::String(s2)) = (self, val) {
            s1.as_ref(program, gc) == s2.as_ref(program, gc)
        } else {
            self == val
        }
    }

    pub fn coerced_cmp(
        &self,
        val: &GribValue,
        program: &Program,
        runtime: &Runtime,
    ) -> Option<Ordering> {
        if self.is_number() || val.is_number() {
            self.cast_num(program, &runtime.gc)
                .partial_cmp(&val.cast_num(program, &runtime.gc))
        } else {
            self.as_str(program, runtime)
                .partial_cmp(&val.as_str(program, runtime))
        }
    }

    pub fn to_str(&self, runtime: &mut Runtime) -> GribString {
        match self {
            Self::Nil => GribString::Static("nil"),
            Self::Callable(fnc) => GribString::Static(match fnc {
                Callable::Native(_) => "[native]",
                Callable::Procedure { .. } => "[proc]",
                Callable::Lambda { .. } => "[lambda]",
            }),
            Self::Bool(b) => GribString::Static(if *b { "true" } else { "false" }),
            Self::HeapValue(ind) => GribString::Static(match runtime.gc.heap_val(*ind) {
                Some(HeapValue::Array(_)) => "[array]",
                Some(HeapValue::Hash(_)) => "[hash]",
                _ => "[stack object]",
            }),
            Self::String(s) => s.clone(),
            Self::Number(n) => runtime.alloc_str(n.to_string()),
            Self::ModuleObject(_) => GribString::Static("[module]"),
            Self::Error(_) => GribString::Static("[error]"),
        }
    }

    pub fn as_str<'a>(&'a self, program: &'a Program, runtime: &'a Runtime) -> Cow<'a, str> {
        match self {
            Self::Nil => "nil".into(),
            Self::Callable(fnc) => match fnc {
                Callable::Native(_) => "[native]",
                Callable::Procedure { .. } => "[proc]",
                Callable::Lambda { .. } => "[lambda]",
            }
            .into(),
            Self::Bool(b) => if *b { "true" } else { "false" }.into(),
            Self::HeapValue(ind) => match runtime.gc.heap_val(*ind) {
                Some(HeapValue::Array(_)) => "[array]",
                Some(HeapValue::Hash(_)) => "[hash]",
                _ => "[stack object]",
            }
            .into(),
            Self::String(s) => s.as_ref(program, &runtime.gc).unwrap_or_default().into(),
            Self::Number(n) => n.to_string().into(),
            Self::ModuleObject(_) => "[module]".into(),
            Self::Error(_) => "[error]".into(),
        }
    }

    pub fn cast_ind(&self, program: &Program, gc: &Gc) -> Option<usize> {
        Some(self.cast_num(program, gc).trunc())
            .filter(|&i| i.is_finite() && i >= 0. && i <= (usize::MAX as f64))
            .map(|i| i as usize)
    }

    pub fn truthy(&self, program: &Program, gc: &Gc) -> bool {
        match self {
            GribValue::Callable(_) | GribValue::ModuleObject(_) => true,
            GribValue::Number(n) => *n != 0.0,
            GribValue::Nil | GribValue::Error(_) => false,
            GribValue::HeapValue(_) => true,
            GribValue::Bool(b) => *b,
            GribValue::String(s) => s.as_ref(program, gc).filter(|r| !r.is_empty()).is_some(),
        }
    }

    pub fn cast_num(&self, program: &Program, gc: &Gc) -> f64 {
        match self {
            Self::Nil => 0.0,
            Self::Callable(_)
            | Self::ModuleObject(_)
            | Self::HeapValue(_)
            | GribValue::Error(_) => f64::NAN,
            Self::Number(n) => *n,
            Self::String(s) => s
                .as_ref(program, gc)
                .and_then(|s| s.cast_num())
                .unwrap_or(f64::NAN),
            Self::Bool(b) => (*b as i32) as f64,
        }
    }
}

impl Default for GribValue {
    fn default() -> Self {
        Self::Nil
    }
}

impl From<f64> for GribValue {
    fn from(f: f64) -> Self {
        GribValue::Number(f)
    }
}

impl From<usize> for GribValue {
    fn from(f: usize) -> Self {
        GribValue::HeapValue(f)
    }
}

impl From<Callable> for GribValue {
    fn from(f: Callable) -> Self {
        GribValue::Callable(f)
    }
}

impl From<GribString> for GribValue {
    fn from(s: GribString) -> Self {
        GribValue::String(s)
    }
}

impl From<bool> for GribValue {
    fn from(b: bool) -> Self {
        GribValue::Bool(b)
    }
}

impl From<StaticValue> for GribValue {
    fn from(s: StaticValue) -> Self {
        match s {
            StaticValue::Function(fnc) => GribValue::Callable(fnc),
            StaticValue::Module(module) => GribValue::ModuleObject(module),
        }
    }
}
