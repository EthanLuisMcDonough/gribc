pub mod expression;
pub mod function;
pub mod module;
pub mod native;
pub mod object;
pub mod statement;

use std::path::Path;

pub use self::expression::*;
pub use self::function::*;
pub use self::module::*;
pub use self::native::*;
pub use self::object::*;
pub use self::statement::*;

pub use runtime::native_fn::{NativeFunction, NativePackage};

pub type Block = Vec<Node>;
pub type ModuleStore = Vec<CustomModule>;
/*pub trait Package {
    fn has_function(&self, name: &str) -> bool;
    fn get_functions<'a>(&'a self) -> HashSet<&'a str>;
}

crate::keyword_map!(NativeConsolePackage {
    Println -> "println",
    Readline -> "readline",
});

crate::keyword_map!(NativeFmtPackage {
    Atof -> "atof",
    Atoi -> "atoi",
});

crate::keyword_map!(NativeMathPackage {
    Sin -> "sin",
    Cos -> "cos",
    Tan -> "tan",
    Asin -> "asin",
    Acos -> "acos",
    Atan -> "atan",
    Sqrt -> "sqrt",
    Pow -> "pow",
    Ln -> "ln",
    Log -> "log",
    Round -> "round",
    Floor -> "floor",
    Ceil -> "ceil",
    MathConst -> "mathConst",
});

crate::keyword_map!(NativePackage {
    Fmt -> "fmt",
    Math -> "math",
    Console -> "console",
});

impl NativePackage {
    pub fn raw_names(&self) -> &'static [&'static str] {
        match self {
            Self::Console => NativeConsolePackage::MEMBERS,
            Self::Fmt => NativeFmtPackage::MEMBERS,
            Self::Math => NativeMathPackage::MEMBERS,
        }
    }
}
*//*
impl Package for NativePackage {
    fn get_functions<'a>(&'a self) -> HashSet<&'a str> {
        self.raw_names().iter().map(|f| *f).collect()
    }

    fn has_function(&self, name: &str) -> bool {
        self.get_functions().contains(&name)
    }
}
*/

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Program {
    pub modules: ModuleStore,
    pub imports: Vec<Import>,
    pub functions: Vec<Procedure>,
    pub lambdas: Vec<Lambda>,
    pub autoprops: Vec<AutoProp>,
    pub body: Block,
}

impl Program {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            body: Vec::new(),
            functions: Vec::new(),
            imports: Vec::new(),
            lambdas: Vec::new(),
            autoprops: Vec::new(),
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
    Declaration(Declaration),
    Return(Expression),
    Break,
    Continue,
}
