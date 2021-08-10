use super::{Expression, LambdaBody};
use location::Located;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum LocatedOr<T, E> {
    Located(Located<T>),
    Or(E),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SetProp {
    pub param: String,
    pub block: LambdaBody,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AutoProp {
    pub get: Option<LocatedOr<String, LambdaBody>>,
    pub set: Option<LocatedOr<String, SetProp>>,
    pub capture: HashSet<String>,
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
            capture: HashSet::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum ObjectValue {
    AutoProp(usize),
    Expression(Expression),
}

pub type Hash = HashMap<String, ObjectValue>;
