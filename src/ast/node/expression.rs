use super::{module::Module, Hash};
use location::{Located, Location};
use operators::{Assignment, Binary, Unary};
use runtime::values::Callable;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Assignable {
    Identifier(Located<usize>),
    Stack(StackPointer),
    IndexAccess {
        item: Box<Expression>,
        index: Box<Expression>,
    },
    PropertyAccess {
        item: Box<Expression>,
        property: usize,
    },
}

/// Values known during static analysis
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum StaticValue {
    Function(Callable),
    Module(Module),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum StackPointer {
    StackOffset(usize),
    CaptureIndex(usize),
}

impl Default for StackPointer {
    fn default() -> Self {
        Self::StackOffset(0)
    }
}

/// Accessible variable value during runtime
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum RuntimeValue {
    Static(StaticValue),
    Stack(StackPointer),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Expression {
    Binary {
        op: Binary,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Assignment {
        op: Assignment,
        left: Assignable,
        right: Box<Expression>,
    },
    Unary {
        op: Unary,
        expr: Box<Expression>,
    },

    Bool(bool),
    Number(f64),
    String(usize),
    Identifier(Located<usize>),

    ArrayCreation(Vec<Expression>),
    Hash(Hash),
    MutableHash(Hash),

    FunctionCall {
        function: Box<Expression>,
        args: Vec<Expression>,
    },
    Lambda(usize),

    IndexAccess {
        item: Box<Expression>,
        index: Box<Expression>,
    },
    PropertyAccess {
        item: Box<Expression>,
        property: usize,
    },

    Nil,
    This {
        start: Location,
        end: Location,
    },

    Value(RuntimeValue),
}

impl Expression {
    pub fn is_statement(&self) -> bool {
        match self {
            Expression::FunctionCall { .. } | Expression::Assignment { .. } => true,
            _ => false,
        }
    }
}
