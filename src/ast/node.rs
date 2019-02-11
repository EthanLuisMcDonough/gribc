use super::Block;
use location::Located;
use operators::{Assignment, Binary, Unary};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ConditionBodyPair {
    pub condition: Expression,
    pub block: Block,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Declaration {
    pub identifier: String,
    pub value: Expression,
}

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
    Lambda {
        param_list: Vec<String>,
        body: Block,
    },
    Hash(HashMap<String, Expression>),
    MutableHash(HashMap<String, Expression>),
    Nil,
    Args,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Node {
    Expression(Expression),
    Block(Block),
    LogicChain {
        if_block: ConditionBodyPair,
        elseifs: Vec<ConditionBodyPair>,
        else_block: Option<Block>,
    },
    While(ConditionBodyPair),
    For {
        declaration: Option<Declaration>,
        condition: Option<Expression>,
        increment: Option<Expression>,
        body: Block,
    },
    Procedure {
        identifier: String,
        param_list: Vec<String>,
        body: Block,
    },
    Declaration(Declaration),
    Return(Expression),
    Break,
    Continue,
}
