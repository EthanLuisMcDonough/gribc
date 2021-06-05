use ast::node::*;
use std::rc::Rc;
use std::collections::HashMap;
use std::cell::RefCell;

pub trait Callable {
    fn call<'a>(&self, args: Vec<GribValue<'a>>) -> GribValue<'a>;
}

// Modules
pub enum NativeReference {
    Fmt(NativeFmtPackage),
    Math(NativeMathPackage),
    Console(NativeConsolePackage),
}

impl From<NativeFmtPackage> for NativeReference {
    fn from(f: NativeFmtPackage) -> Self { Self::Fmt(f) }
}
impl From<NativeMathPackage> for NativeReference {
    fn from(f: NativeMathPackage) -> Self { Self::Math(f) }
}
impl From<NativeConsolePackage> for NativeReference {
    fn from(f: NativeConsolePackage) -> Self { Self::Console(f) }
}

// Lambdas
#[derive(Clone)]
pub struct LambdaValue<'a> {
    body: &'a Lambda,
    binding: GribValue<'a>,
}

// Hashes
#[derive(Clone)]
pub struct SetPropertyValue<'a> {
    body: &'a LambdaBody,
    value_name: &'a str,
    binding: GribValue<'a>,
}

#[derive(Clone)]
pub struct GetPropertyValue<'a> {
    body: &'a LambdaBody,
    binding: GribValue<'a>,
}

#[derive(Clone)]
pub struct AutoPropValue<'a> {
    get: Option<GetPropertyValue<'a>>,
    set: Option<SetPropertyValue<'a>>,
}

#[derive(Clone)]
pub enum HashPropertyValue<'a> {
    AutoProp(AutoPropValue<'a>),
    Value(GribValue<'a>),
}

#[derive(Clone)]
pub struct HashValue<'a> {
    mutable: bool,
    values: HashMap<String, HashPropertyValue<'a>>
}

impl<'a> From<AutoPropValue<'a>> for HashPropertyValue<'a> {
    fn from(prop: AutoPropValue<'a>) -> Self {
        HashPropertyValue::AutoProp(prop)
    }
}

impl<'a> From<GribValue<'a>> for HashPropertyValue<'a> {
    fn from(prop: GribValue<'a>) -> Self {
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
    Array(Vec<GribValue<'a>>),
    Hash(HashValue<'a>),
}

#[derive(Clone)]
pub enum ModuleObject<'a> {
    Native(NativePackage),
    Custom(&'a CustomModule)
} 

#[derive(Clone)]
pub enum GribValue<'a> {
    Nil,
    Number(f64),
    String(String),
    Callable(&'a dyn Callable),
    ModuleObject(ModuleObject<'a>),
    HeapValue(usize),
}

impl<'a> From<f64> for GribValue<'a> {
    fn from(f: f64) -> GribValue<'a> {
        GribValue::Number(f)
    }
}

impl<'a> From<String> for GribValue<'a> {
    fn from(s: String) -> GribValue<'a> { 
        GribValue::String(s)
    }
}

impl<'a> Default for GribValue<'a> {
    fn default() -> Self { Self::Nil }
}