use location::{Located, Location};
use operators::{Assignment, Binary, Unary};
use std::collections::{HashMap, HashSet};
use std::path::{PathBuf, Path};

pub type Block = Vec<Node>;
pub type ModuleStore = HashMap<PathBuf, CustomModule>;

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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum LambdaBody {
    ImplicitReturn(Box<Expression>),
    Block(Block),
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
pub struct Procedure {
    pub identifier: Located<String>,
    pub param_list: Parameters,
    pub body: Block,
    pub public: bool,
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
        body: LambdaBody,
    },
    Hash(Hash),
    MutableHash(Hash),
    Nil,
    Args,
    This,
}

impl Expression {
    pub fn is_statement(&self) -> bool {
        match self {
            Expression::FunctionCall { .. } | Expression::Assignment { .. } => true,
            _ => false,
        }
    }
}

pub trait Package {
    fn has_function(&self, name: &str) -> bool;
    fn get_functions<'a>(&'a self) -> HashSet<&'a str>;
}

crate::keyword_map!(NativePackage {
    Fmt -> "fmt",
    Math -> "math",
    Console -> "console",
});

impl NativePackage {
    pub fn raw_names(&self) -> &'static [&'static str] {
        match self {
            Self::Console => &["println", "readline"],
            Self::Fmt => &["atof", "atoi"],
            Self::Math => &[
                "sin", "cos", "tan", 
                "asin", "acos", "atan",
                "sqrt", "pow", "ln", 
                "log", "round", "floor", 
                "ceil", "pi", "e"
            ],
        }
    }
}

impl Package for NativePackage {
    fn get_functions<'a>(&'a self) -> HashSet<&'a str> {
        self.raw_names().iter().map(|f| *f).collect()
    }

    fn has_function(&self, name: &str) -> bool {
        self.get_functions().contains(&name)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Module {
    Custom(Located<PathBuf>),
    Native(NativePackage),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Program {
    pub modules: ModuleStore,
    pub body: Block,
}

impl Program {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn has_module(&self, path: &Path) -> bool {
        self.modules.contains_key(path)
    }

    pub fn set_module(&mut self, path: PathBuf, module: CustomModule) {
        self.modules.insert(path, module);
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum ImportKind {
    All,
    ModuleObject(Located<String>),
    List(HashMap<String, (Location, Location)>),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CustomModule {
    pub imports: Vec<Import>,
    pub functions: Vec<Procedure>,
}

impl Package for CustomModule {
    fn get_functions<'a>(&'a self) -> HashSet<&'a str> {
        self.functions.iter().filter(|f| f.public)
            .map(|f| f.identifier.data.as_str()).collect()
    }

    fn has_function(&self, name: &str) -> bool {
        self.functions.iter().filter(|f| f.public)
            .any(|f| f.identifier.data == name)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Import {
    pub module: Module,
    pub kind: ImportKind,
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
    Procedure(Procedure),
    Declaration(Declaration),
    Return(Expression),
    Import(Import),
    Break,
    Continue,
}
