pub mod controlflow;
pub mod expression;
pub mod function;
pub mod indices;
pub mod module;
pub mod object;
pub mod statement;

use crate::location::Located;

pub use self::controlflow::*;
pub use self::expression::*;
pub use self::function::*;
pub use self::indices::*;
pub use self::module::*;
pub use self::object::*;
pub use self::statement::*;

pub use runtime::native_fn::{NativeFunction, NativePackage};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Block {
    pub stmts: Vec<Node>,
    pub allocations: usize,
}

impl Block {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iter(&'_ self) -> impl Iterator<Item = &'_ Node> {
        self.stmts.iter()
    }

    pub fn iter_mut(&'_ mut self) -> impl Iterator<Item = &'_ mut Node> {
        self.stmts.iter_mut()
    }

    pub fn push(&mut self, node: Node) {
        self.stmts.push(node);
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Program {
    pub modules: ModuleStore,
    pub imports: Imports,
    pub procedures: Procedures,
    pub lambdas: Lambdas,
    pub getters: Getters,
    pub setters: Setters,
    pub strings: StringStore,
    pub body: Block,
}

impl Program {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Node {
    Expression(Expression),
    Block(Block),
    LogicChain(LogicChain),
    While(ConditionBodyPair),
    For(ForLoop),
    Declaration(Declaration),
    ControlFlow(Located<FlowBreak>),
}
