use super::{Expression, LambdaBody};
use location::Located;
use std::collections::HashMap;

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
