use super::{module::Module, Hash, Identifier, LambdaInd, StringVal};
use location::{Located, LocationSpan};
use operators::{Assignment, Binary, Unary};
use runtime::{values::Callable, StackOffset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct IndexAccess {
    pub item: Box<Expression>,
    pub index: Box<Expression>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct PropertyAccess {
    pub item: Box<Expression>,
    pub property: Identifier,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Assignable {
    Identifier(Located<Identifier>),
    Offset(StackOffset),
    IndexAccess(IndexAccess),
    PropertyAccess(PropertyAccess),
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
    StackOffset(StackOffset),
}

pub type ExpressionList = Vec<Expression>;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct BinaryExpression {
    pub op: Binary,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct UnaryExpression {
    pub op: Unary,
    pub expr: Box<Expression>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AssignmentExpression {
    pub op: Assignment,
    pub left: Assignable,
    pub right: Box<Expression>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CallExpression {
    pub function: Box<Expression>,
    pub args: Vec<Expression>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Expression {
    Assignment(AssignmentExpression),
    Binary(BinaryExpression),
    Unary(UnaryExpression),

    Bool(bool),
    Number(f64),
    String(StringVal),
    Identifier(Located<Identifier>),

    ArrayCreation(ExpressionList),
    Hash(Hash),
    MutableHash(Hash),

    FunctionCall(CallExpression),
    Lambda(LambdaInd),

    IndexAccess(IndexAccess),
    PropertyAccess(PropertyAccess),

    Nil(LocationSpan),
    This(LocationSpan),

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
