use ast::node::*;
use runtime::memory::Gc;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::cmp::Ordering;

/*pub trait Callable {
    //fn call(&self, gc: &mut Gc, args: Vec<GribValue>) -> GribValue;
    fn call(&self, args: Vec<GribValue>) -> GribValue;
}

pub enum CallableType {
    Lambda,
    Native,
    Procedure,
}*/

#[derive(Clone)]
pub struct LambdaRef {
    pub binding: usize,
    index: usize,
    pub stack: usize,
}

#[derive(Clone)]
pub enum Callable {
    Native(NativeFunction),
    Procedure {
        module: Option<usize>,
        index: usize,
    },
    Lambda {
        binding: usize,
        index: usize,
        stack: usize,
    },
}

impl Callable {
    pub fn call(&self, gc: &mut Gc, program: &Program, args: Vec<GribValue>) -> GribValue {
        match self {
            Callable::Native(n) => n.call(gc, args),
            Callable::Procedure { module, index } => {
                let fnc = if let Some(i) = module {
                    &program.modules[*i].functions[*index]
                } else {
                    &program.functions[*index]
                };

                unimplemented!()
            }
            Callable::Lambda { .. } => {
                unimplemented!()
            }
        }
    }
}

pub struct CapturedStack {}

/*// Modules
#[derive(Clone)]
pub enum NativeReference {
    Fmt(NativeFmtPackage),
    Math(NativeMathPackage),
    Console(NativeConsolePackage),
}

impl NativeReference {
    pub fn name(&self) -> &'static str {
        unimplemented!()
    }
}

impl From<NativeFmtPackage> for NativeReference {
    fn from(f: NativeFmtPackage) -> Self {
        Self::Fmt(f)
    }
}
impl From<NativeMathPackage> for NativeReference {
    fn from(f: NativeMathPackage) -> Self {
        Self::Math(f)
    }
}
impl From<NativeConsolePackage> for NativeReference {
    fn from(f: NativeConsolePackage) -> Self {
        Self::Console(f)
    }
}*/

#[derive(Clone)]
pub enum AccessFunc {
    Callable { index: usize, stack: usize, binding: usize },
    Captured(usize),
}

/*impl AutoPropValue {
    pub fn functions<'a>(&'a self) -> impl Iterator<Item = &'a AccessFunc> {
        self.get.iter().chain(self.set.iter())
    }
}*/

#[derive(Clone)]
pub enum HashPropertyValue {
    AutoProp {
        get: Option<AccessFunc>,
        set: Option<AccessFunc>,
    },
    Value(GribValue),
}

#[derive(Clone)]
pub struct HashValue {
    pub mutable: bool,
    pub values: HashMap<String, HashPropertyValue>,
}

impl From<GribValue> for HashPropertyValue {
    fn from(prop: GribValue) -> Self {
        HashPropertyValue::Value(prop)
    }
}

impl HashValue {
    pub fn insert_property(&mut self, key: String, value: impl Into<HashPropertyValue>) {
        self.values.insert(key, value.into());
    }
}

#[derive(Clone)]
pub enum HeapValue {
    Array(Vec<GribValue>),
    Hash(HashValue),
    String(String),
    CapturedStack(HashMap<String, usize>),
}

#[derive(Clone)]
pub enum ModuleObject {
    Native(NativePackage),
    Custom(usize),
}

#[derive(Clone)]
pub enum GribValue {
    Nil,
    Number(f64),
    Callable(Callable),
    ModuleObject(Module),
    HeapValue(usize),
    Bool(bool),
}

impl GribValue {
    pub fn ptr(&self) -> Option<usize> {
        if let &Self::HeapValue(i) = self {
            Some(i)
        } else {
            None
        }
    }

    pub fn partial_cmp(&self, val: &GribValue, gc: &Gc) -> Option<Ordering> {
        match self {
            //GribValue::Number(n) => *n.partial_cmp(&val.cast_num(gc)),
            /*GribValue::HeapValue(ptr) => {

                gc
            }*/
            //GribValue::Bool()
            _ => unimplemented!(),
        }
    }

    pub fn as_str<'a>(&'a self, gc: &'a Gc) -> Cow<'a, str> {
        match self {
            Self::Nil => "nil".into(),
            Self::Callable(fnc) => match fnc {
                Callable::Native(n) => format!("[native {}()]", n.fn_name()).into(),
                Callable::Procedure { .. } => "[proc]".into(),
                Callable::Lambda { .. } => "[lambda]".into(),
            },
            Self::Bool(b) => if *b { "true" } else { "false" }.into(),
            Self::HeapValue(ind) => match gc.heap_val(*ind) {
                Some(HeapValue::Array(v)) => {
                    let mut joined = "[".to_string();

                    for value in v {
                        joined.push_str(value.as_str(gc).as_ref());
                        joined.push(',');
                    }

                    joined.pop();
                    joined.push(']');

                    joined.into()
                }
                Some(HeapValue::Hash(h)) => {
                    let mut joined = if h.mutable { '$' } else { '#' }.to_string();

                    joined.push('{');
                    for (key, value) in h.values.iter() {
                        joined.push_str(key);
                        joined.push_str("->");

                        match value {
                            HashPropertyValue::AutoProp { .. } => joined.push_str("[auto prop]"),
                            HashPropertyValue::Value(v) => joined.push_str(v.as_str(gc).as_ref()),
                        }

                        joined.push(',')
                    }

                    joined.pop();
                    joined.push('}');

                    joined.into()
                }
                Some(HeapValue::String(s)) => Cow::Borrowed(s),
                _ => "[stack object]".into(),
            },
            Self::Number(n) => n.to_string().into(),
            Self::ModuleObject(_) => "[module]".into(),
        }
    }

    pub fn cast_ind(&self, gc: &Gc) -> Option<usize> {
        Some(self.cast_num(gc).trunc().abs())
            .filter(|i| i.is_finite())
            .map(|i| i as usize)
    }

    pub fn truthy(&self) -> bool {
        match self {
            GribValue::Callable(_) | GribValue::ModuleObject(_) => true,
            GribValue::Number(n) => *n != 0.0,
            GribValue::Nil => false,
            GribValue::HeapValue(_) => true,
            GribValue::Bool(b) => *b,
        }
    }

    pub fn cast_num(&self, gc: &Gc) -> f64 {
        match self {
            Self::Nil => 0.0,
            Self::Callable(_) | Self::ModuleObject(_) => f64::NAN,
            Self::Number(n) => *n,
            Self::HeapValue(ind) => gc
                .get_str(*ind)
                .and_then(|s| s.parse().ok())
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
