use super::{Hash, Lambda};
use location::Located;
use operators::{Assignment, Binary, Unary};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Assignable {
    Identifier(Located<String>),
    IndexAccess {
        item: Box<Expression>,
        index: Box<Expression>,
    },
    PropertyAccess {
        item: Box<Expression>,
        property: String,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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
    String(String),
    Identifier(Located<String>),
    ArrayCreation(Vec<Expression>),
    FunctionCall {
        function: Box<Expression>,
        args: Vec<Expression>,
    },
    IndexAccess {
        item: Box<Expression>,
        index: Box<Expression>,
    },
    PropertyAccess {
        item: Box<Expression>,
        property: String,
    },
    Lambda(Lambda),
    Hash(Hash),
    MutableHash(Hash),
    Nil,
    Args,
    This,
}

impl Expression {
    pub fn is_statement(&self) -> bool {
        match self {
            Expression::FunctionCall { .. } | Expression::Assignment { .. } => true,
            _ => false,
        }
    }
}
