mod callable;
mod hash;
mod string;

use ast::node::*;
use runtime::memory::{Gc, Runtime};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;

pub use self::callable::*;
pub use self::hash::*;
pub use self::string::*;

/*pub trait Callable {
    //fn call(&self, gc: &mut Gc, args: Vec<GribValue>) -> GribValue;
    fn call(&self, args: Vec<GribValue>) -> GribValue;
}

pub enum CallableType {
    Lambda,
    Native,
    Procedure,
}*/

/*impl AutoPropValue {
    pub fn functions<'a>(&'a self) -> impl Iterator<Item = &'a AccessFunc> {
        self.get.iter().chain(self.set.iter())
    }
}*/

#[derive(Clone)]
pub enum HeapValue {
    Array(Vec<GribValue>),
    Hash(HashValue),
    String(String),
    CapturedStack(HashMap<usize, usize>),
}

#[derive(Clone)]
pub enum ModuleObject {
    Native(NativePackage),
    Custom(usize),
}

pub fn float_to_ind(f: f64) -> Option<usize> {
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
}

#[derive(Clone)]
pub enum GribValue {
    Nil,
    Number(f64),
    String(GribString),
    Callable(Callable),
    ModuleObject(Module),
    HeapValue(usize),
    Bool(bool),
}

impl GribValue {
    pub fn ptr(&self) -> Option<usize> {
        match self {
            Self::HeapValue(i) | Self::String(GribString::Heap(i)) => Some(*i),
            _ => None,
        }
    }

    pub fn partial_cmp(&self, val: &GribValue, program: &Program, gc: &Gc) -> Option<Ordering> {
        if let Some(string) = gc.try_get_string(self, program) {
            unimplemented!()
        } else {
            self.cast_num(program, gc)
                .partial_cmp(&val.cast_num(program, gc))
        }
    }

    pub fn to_str(&self, runtime: &mut Runtime, program: &Program) -> GribString {
        match self {
            Self::Nil => GribString::Static("nil"),
            Self::Callable(fnc) => match fnc {
                Callable::Native(n) => runtime.alloc_str(format!("[native {}()]", n.fn_name())),
                Callable::Procedure { .. } => GribString::Static("[proc]"),
                Callable::Lambda { .. } => GribString::Static("[lambda]"),
            },
            Self::Bool(b) => GribString::Static(if *b { "true" } else { "false" }),
            Self::HeapValue(ind) => match runtime.gc.heap_val(*ind) {
                Some(HeapValue::Array(v)) => {
                    let array = v.clone();
                    let s = array_to_string(&array, program, runtime);
                    runtime.alloc_str(s)
                }
                Some(HeapValue::Hash(h)) => {
                    let mutable = h.is_mutable();
                    let s = values_to_string(
                        h.clone().into_values(runtime, program, *ind),
                        mutable,
                        program,
                        runtime,
                    );
                    runtime.alloc_str(s)
                }
                _ => GribString::Static("[stack object]"),
            },
            Self::String(s) => s.clone(),
            Self::Number(n) => runtime.alloc_str(n.to_string()),
            Self::ModuleObject(_) => GribString::Static("[module]"),
        }
    }

    pub fn as_str<'a>(&'a self, program: &'a Program, runtime: &'a mut Runtime) -> Cow<'a, str> {
        match self {
            Self::Nil => "nil".into(),
            Self::Callable(fnc) => match fnc {
                Callable::Native(n) => format!("[native {}()]", n.fn_name()).into(),
                Callable::Procedure { .. } => "[proc]".into(),
                Callable::Lambda { .. } => "[lambda]".into(),
            },
            Self::Bool(b) => if *b { "true" } else { "false" }.into(),
            Self::HeapValue(ind) => match runtime.gc.heap_val(*ind) {
                Some(HeapValue::Array(v)) => {
                    let array = v.clone();
                    array_to_string(&array, program, runtime).into()
                }
                Some(HeapValue::Hash(h)) => {
                    let mutable = h.is_mutable();
                    values_to_string(
                        h.clone().into_values(runtime, program, *ind),
                        mutable,
                        program,
                        runtime,
                    )
                    .into()
                }
                _ => "[stack object]".into(),
            },
            Self::String(s) => s.as_ref(program, &runtime.gc).unwrap_or_default().into(),
            Self::Number(n) => n.to_string().into(),
            Self::ModuleObject(_) => "[module]".into(),
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
            GribValue::Nil => false,
            GribValue::HeapValue(_) => true,
            GribValue::Bool(b) => *b,
            GribValue::String(s) => s.as_ref(program, gc).filter(|r| r.is_empty()).is_some(),
        }
    }

    pub fn cast_num(&self, program: &Program, gc: &Gc) -> f64 {
        match self {
            Self::Nil => 0.0,
            Self::Callable(_) | Self::ModuleObject(_) | Self::HeapValue(_) => f64::NAN,
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
