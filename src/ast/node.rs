use super::Block;
use location::Located;
use operators::{Assignment, Binary, Unary};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ConditionBodyPair {
    pub condition: Expression,
    pub block: Block,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Declarator {
    pub identifier: Located<String>,
    pub value: Expression,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Declaration {
    pub declarations: Vec<Declarator>,
    pub mutable: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Parameters {
    pub params: HashSet<String>,
    pub vardic: Option<String>,
}

impl Parameters {
    pub fn new() -> Self {
        Parameters {
            params: HashSet::new(),
            vardic: None,
        }
    }

    pub fn all_params<'a>(&'a self) -> impl Iterator<Item = &'a str> {
        self.params.iter().chain(self.vardic.iter()).map(|s| &**s)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum LocatedOr<T, E> {
    Located(Located<T>),
    Or(E),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SetProp {
    pub param: String,
    pub block: Block,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AutoProp {
    pub get: Option<LocatedOr<String, Block>>,
    pub set: Option<LocatedOr<String, SetProp>>,
}

impl AutoProp {
    pub fn new() -> Self {
        AutoProp {
            get: None,
            set: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum ObjectValue {
    AutoProp(AutoProp),
    Expression(Expression),
}

pub type Hash = HashMap<String, ObjectValue>;

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
        param_list: Parameters,
        body: Block,
    },
    Hash(Hash),
    MutableHash(Hash),
    Nil,
    Args,
}

impl Expression {
    pub fn is_statement(&self) -> bool {
        match self {
            Expression::FunctionCall { .. } | Expression::Assignment { .. } => true,
            _ => false,
        }
    }
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
        identifier: Located<String>,
        param_list: Parameters,
        body: Block,
    },
    Declaration(Declaration),
    Return(Expression),
    Break,
    Continue,
}
