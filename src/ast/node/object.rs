use super::{Expression, LambdaBody, RuntimeValue, StringVal, GetPropInd, Identifier, SetPropInd};
use location::Located;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum CapturedVars {
    Names(Vec<Identifier>),
    Offsets(Vec<usize>),
}

impl Default for CapturedVars {
    fn default() -> Self {
        Self::Names(Vec::new())
    }
}

/// Setter function
/// Stored in program struct
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SetProp {
    /// Param name
    pub param: Identifier,

    /// Whether the parameter is a captured variable
    pub param_captured: bool,
    pub block: LambdaBody,

    /// Captured variables
    /// As with lambdas, identifier names are
    /// stored after first pass.  Stack offsets
    /// are stored after second pass
    pub capture: CapturedVars,
}

impl Default for SetProp {
    fn default() -> Self {
        Self {
            param: 0.into(),
            param_captured: false,
            block: LambdaBody::default(),
            capture: CapturedVars::default(),
        }
    }
}

impl SetProp {
    pub fn new(param: Identifier, block: LambdaBody) -> Self {
        Self {
            param,
            param_captured: false,
            block,
            capture: CapturedVars::default(),
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
    pub capture: CapturedVars,
}

impl GetProp {
    pub fn new(block: LambdaBody) -> Self {
        Self {
            block,
            capture: CapturedVars::default(),
        }
    }
}

/// Value representing a getter value in hashes
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum GetAutoPropValue {
    /// Identifier in first pass
    String(Located<Identifier>),

    /// Index to getter or setter in program struct
    Lambda(GetPropInd),

    /// Static value that are detected during second pass
    /// Function/import values only
    /// Used only for getters
    Value(RuntimeValue),
}


/// Value representing a setter value in hashes
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum SetAutoPropValue {
    /// Identifier in first pass
    String(Located<Identifier>),

    /// Stack offset after second pass
    StackOffset(usize),

    /// Index to getter or setter in program struct
    Lambda(SetPropInd),
}

/// AST value for an "auto-property" (getter/setter pair)
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct AutoProp {
    pub get: Option<GetAutoPropValue>,
    pub set: Option<SetAutoPropValue>,
}

impl AutoProp {
    pub fn new() -> Self {
        AutoProp {
            get: None,
            set: None,
        }
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
pub type Hash = HashMap<StringVal, ObjectValue>;
