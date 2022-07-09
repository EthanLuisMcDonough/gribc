pub mod controlflow;
pub mod expression;
pub mod function;
pub mod module;
pub mod object;
pub mod statement;

use std::path::Path;

pub use self::controlflow::*;
pub use self::expression::*;
pub use self::function::*;
pub use self::module::*;
pub use self::object::*;
pub use self::statement::*;

pub use runtime::native_fn::{NativeFunction, NativePackage};

pub type ModuleStore = Vec<CustomModule>;

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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Program {
    pub modules: ModuleStore,
    pub imports: Vec<Import>,
    pub functions: Vec<Procedure>,
    pub lambdas: Vec<Lambda>,
    pub getters: Vec<GetProp>,
    pub setters: Vec<SetProp>,
    pub strings: Vec<String>,
    pub body: Block,
}

impl Program {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            body: Block::default(),
            functions: Vec::new(),
            imports: Vec::new(),
            lambdas: Vec::new(),
            getters: Vec::new(),
            setters: Vec::new(),
            strings: Vec::new(),
        }
    }

    pub fn has_module(&self, path: &Path) -> Option<usize> {
        for i in 0..self.modules.len() {
            if self.modules[i].path == path {
                return Some(i);
            }
        }

        None
    }

    pub fn set_module(&mut self, module: CustomModule) -> usize {
        let ind = self.modules.len();
        self.modules.push(module);
        ind
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
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
    Declaration(Declaration),
    ControlFlow(FlowBreak),
}
