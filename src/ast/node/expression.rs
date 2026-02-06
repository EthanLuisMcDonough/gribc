use super::{module::Module, Hash, Identifier, LambdaInd, StringVal};
use location::{Located, Location};
use operators::{Assignment, Binary, Unary};
use runtime::values::Callable;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct IndexAccess {
    pub item: Box<Expression>,
    pub index: Box<Expression>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Assignable {
    Identifier(Located<Identifier>),
    Offset(usize),
    IndexAccess {
        item: Box<Expression>,
        index: Box<Expression>,
    },
    PropertyAccess {
        item: Box<Expression>,
        property: Identifier,
    },
}

/// Values known during static analysis
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum StaticValue {
    Function(Callable),
    Module(Module),
}

/// Accessible variable value during runtime
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum RuntimeValue {
    Static(StaticValue),
    StackOffset(usize),
}

pub type ExpressionList = Vec<Expression>;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct PropertyAccess {
    pub item: Box<Expression>,
    pub property: Identifier,
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
    String(StringVal),
    Identifier(Located<Identifier>),

    ArrayCreation(ExpressionList),
    Hash(Hash),
    MutableHash(Hash),

    FunctionCall {
        function: Box<Expression>,
        args: Vec<Expression>,
    },
    Lambda(LambdaInd),

    IndexAccess(IndexAccess),
    PropertyAccess(PropertyAccess),

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

    pub fn is_operation(&self) -> bool {
        match self {
            Expression::Assignment { .. }
            | Expression::Binary { .. }
            | Expression::Unary { .. } => true,
            _ => false,
        }
    }
}
