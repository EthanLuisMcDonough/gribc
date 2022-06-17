use super::{Block, Expression, Module};
use location::{Located, Location};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ConditionBodyPair {
    pub condition: Expression,
    pub block: Block,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Declarator {
    pub identifier: Located<usize>,
    pub value: Expression,
    pub captured: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Declaration {
    pub declarations: Vec<Declarator>,
    pub mutable: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum ImportKind {
    All,
    ModuleObject(Located<usize>),
    List(HashMap<usize, (Location, Location)>),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Import {
    pub module: Module,
    pub kind: ImportKind,
}
