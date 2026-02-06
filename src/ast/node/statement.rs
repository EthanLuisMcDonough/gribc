use super::{Block, Expression, Module, Identifier};
use location::Located;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ConditionBodyPair {
    pub condition: Expression,
    pub block: Block,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct LogicChain {
    pub if_block: ConditionBodyPair,
    pub elseifs: Vec<ConditionBodyPair>,
    pub else_block: Option<Block>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ForLoop {
    pub declaration: Option<Declaration>,
    pub condition: Option<Expression>,
    pub increment: Option<Expression>,
    pub body: Block,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Declarator {
    pub identifier: Located<Identifier>,
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
    ModuleObject(Located<Identifier>),
    List(Vec<Located<Identifier>>),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Import {
    pub module: Located<Module>,
    pub kind: ImportKind,
}
