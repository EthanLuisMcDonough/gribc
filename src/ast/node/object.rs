use super::{Expression, LambdaBody, RuntimeValue, StackPointer};
use location::Located;
use std::collections::HashMap;

/// Setter function
/// Stored in program struct
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct SetProp {
    /// Param name
    pub param: usize,

    /// Whether the parameter is a captured variable
    pub param_captured: bool,
    pub block: LambdaBody,

    /// Captured variables
    /// As with lambdas, identifier names are
    /// stored after first pass.  Stack offsets
    /// are stored after second pass
    pub capture: Vec<StackPointer>,
}

impl SetProp {
    pub fn new(param: usize, block: LambdaBody) -> Self {
        Self {
            param,
            param_captured: false,
            block,
            capture: Vec::new(),
        }
    }
}

/// Getter function
/// Stored in program struct
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct GetProp {
    pub block: LambdaBody,

    /// Captured variables
    /// Once again, identifier names in first pass
    /// Stack offsets after second pass
    pub capture: Vec<StackPointer>,
}

impl GetProp {
    pub fn new(block: LambdaBody) -> Self {
        Self {
            block,
            capture: Vec::new(),
        }
    }
}

/// Value representing a getter or setter value in hashes
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum AutoPropValue {
    /// Identifier in first pass
    /// Stack offset after second pass
    String(Located<usize>),

    /// Index to getter or setter in program struct
    Lambda(usize),

    /// Static value that are detected during second pass
    /// Function/import values only
    /// Used only for getters
    Value(RuntimeValue),
}

/// AST value for an "auto-property" (getter/setter pair)
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct AutoProp {
    pub get: Option<AutoPropValue>,
    pub set: Option<AutoPropValue>,
}

impl AutoProp {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Hash keys
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum ObjectValue {
    AutoProp(AutoProp),
    Expression(Expression),
}

/// Object/hash literal
/// Keys point to strings stored in the program struct
pub type Hash = HashMap<usize, ObjectValue>;
