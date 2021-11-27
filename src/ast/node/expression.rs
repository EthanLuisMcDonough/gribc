use super::{Hash, Lambda};
use location::Located;
use operators::{Assignment, Binary, Unary};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Assignable {
    Identifier(Located<usize>),
    IndexAccess {
        item: Box<Expression>,
        index: Box<Expression>,
    },
    PropertyAccess {
        item: Box<Expression>,
        property: usize,
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
    Args,
    This,

    StackRef(usize),
}
impl Expression {
    pub fn is_statement(&self) -> bool {
        match self {
            Expression::FunctionCall { .. } | Expression::Assignment { .. } => true,
            _ => false,
        }
    }
}
