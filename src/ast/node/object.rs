use super::{Block, Expression, LambdaBody};
use location::Located;
use std::collections::{HashMap, HashSet};

/*#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum LocatedOr<T, E> {
    Located(Located<T>),
    Or(E),
}*/

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SetProp {
    pub param: usize,
    pub block: LambdaBody,
    pub capture: Vec<usize>,
}

impl Default for SetProp {
    fn default() -> Self {
        Self::new(0, LambdaBody::Block(Block::default()))
    }
}

impl SetProp {
    pub fn new(param: usize, block: LambdaBody) -> Self {
        Self {
            param,
            block,
            capture: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GetProp {
    pub block: LambdaBody,
    pub capture: HashSet<usize>,
}

impl Default for GetProp {
    fn default() -> Self {
        Self::new(LambdaBody::Block(vec![]))
    }
}

impl GetProp {
    pub fn new(block: LambdaBody) -> Self {
        Self {
            block,
            capture: HashSet::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum AutoPropValue {
    String(Located<usize>),
    Lambda(usize),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AutoProp {
    pub get: Option<AutoPropValue>,
    pub set: Option<AutoPropValue>,
}

impl Default for AutoProp {
    fn default() -> Self {
        Self::new()
    }
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

pub type Hash = HashMap<usize, ObjectValue>;
