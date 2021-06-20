use ast::node::*;
use runtime::memory::Gc;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::path::PathBuf;

pub trait Callable {
    //fn call(&self, gc: &mut Gc, args: Vec<GribValue>) -> GribValue;
    fn call(&self, args: Vec<GribValue>) -> GribValue;
}

pub enum CallableType {
    Lambda,
    Native,
    Procedure,
}
pub struct Callable {
    pub binding: GribValue,
    pub kind: CallableType,
    pub 
}

// Modules
#[derive(Clone)]
pub enum NativeReference {
    Fmt(NativeFmtPackage),
    Math(NativeMathPackage),
    Console(NativeConsolePackage),
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
}

// Lambdas
#[derive(Clone)]
pub struct LambdaValue<'a> {
    body: &'a Lambda,
    binding: GribValue,
}

// Hashes
#[derive(Clone)]
pub struct SetPropertyValue<'a> {
    body: &'a LambdaBody,
    value_name: &'a str,
    binding: GribValue,
}

#[derive(Clone)]
pub struct GetPropertyValue<'a> {
    body: &'a LambdaBody,
    binding: GribValue,
}

#[derive(Clone)]
pub struct AutoPropValue<'a> {
    get: Option<GetPropertyValue<'a>>,
    set: Option<SetPropertyValue<'a>>,
}

#[derive(Clone)]
pub enum HashPropertyValue<'a> {
    AutoProp(AutoPropValue<'a>),
    Value(GribValue),
}

#[derive(Clone)]
pub struct HashValue<'a> {
    mutable: bool,
    values: HashMap<String, HashPropertyValue<'a>>,
}

impl<'a> From<AutoPropValue<'a>> for HashPropertyValue<'a> {
    fn from(prop: AutoPropValue<'a>) -> Self {
        HashPropertyValue::AutoProp(prop)
    }
}

impl<'a> From<GribValue> for HashPropertyValue<'a> {
    fn from(prop: GribValue) -> Self {
        HashPropertyValue::Value(prop)
    }
}

impl<'a> HashValue<'a> {
    pub fn insert_property(&mut self, key: String, value: impl Into<HashPropertyValue<'a>>) {
        self.values.insert(key, value.into());
    }
}

#[derive(Clone)]
pub enum HeapValue<'a> {
    Array(Vec<GribValue>),
    Hash(HashValue<'a>),
}

#[derive(Clone)]
pub enum ModuleObject {
    Native(NativePackage),
    Custom(PathBuf),
}

#[derive(Clone)]
pub enum GribValue {
    Nil,
    Number(f64),
    String(String),
    Callable(Box<dyn Callable>),
    ModuleObject(ModuleObject),
    HeapValue(usize),
}

impl GribValue {
    pub fn ptr(&self) -> Option<usize> {
        if let &Self::HeapValue(i) = self {
            Some(i)
        } else { None }
    }

    pub fn as_str<'a>(&'a self) -> Cow<'a, str> {
        match self {
            Self::Nil => "nil".into(),
            Self::Callable(_) => "[callable]".into(),
            Self::HeapValue(_) => "[object]".into(),
            Self::String(s) => Cow::Borrowed(s),
            Self::Number(n) => n.to_string().into(),
            Self::ModuleObject(_) => "[module]".into(),
        }
    }

    pub fn cast_num(&self) -> f64 {
        match self {
            Self::Nil => 0.0,
            Self::Callable(_) | Self::HeapValue(_) | Self::ModuleObject(_) => f64::NAN,
            Self::Number(n) => *n,
            Self::String(s) => s.parse().unwrap_or(f64::NAN),
        }
    }
}

impl From<f64> for GribValue {
    fn from(f: f64) -> GribValue {
        GribValue::Number(f)
    }
}

impl From<String> for GribValue {
    fn from(s: String) -> GribValue {
        GribValue::String(s)
    }
}

impl Default for GribValue {
    fn default() -> Self {
        Self::Nil
    }
}

impl ToString for GribValue {
    fn to_string(&self) -> String {
        self.as_str().into_owned()
    }
}
