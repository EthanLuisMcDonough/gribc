use super::{Block, Expression};
use location::Located;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Param {
    pub name: usize,
    pub captured: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Parameters {
    pub params: Vec<Param>,
    pub vardic: Option<Param>,
}

impl Parameters {
    pub fn new() -> Self {
        Parameters {
            params: Vec::new(),
            vardic: None,
        }
    }

    pub fn contains(&self, name: usize) -> bool {
        self.all_params().any(|p| p.name == name)
    }

    pub fn all_params(&'_ self) -> impl Iterator<Item = &'_ Param> {
        self.params.iter().chain(self.vardic.iter())
    }

    pub(in ast) fn all_params_mut(&'_ mut self) -> impl Iterator<Item = &'_ mut Param> {
        self.params.iter_mut().chain(self.vardic.iter_mut())
    }

    /// This method should only be called while parsing the initial AST
    pub(in ast) fn try_add(&mut self, name: usize) -> bool {
        let param = Param {
            name,
            captured: false,
        };
        if self.params.contains(&param) {
            false
        } else {
            self.params.push(param);
            true
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum LambdaBody {
    ImplicitReturn(Box<Expression>),
    Block(Block),
}

impl Default for LambdaBody {
    fn default() -> Self {
        Self::Block(Block::new())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Procedure {
    pub identifier: Located<usize>,
    pub param_list: Parameters,
    pub body: Block,
    pub public: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Lambda {
    pub param_list: Parameters,
    pub body: LambdaBody,
    pub captured: Vec<usize>,
}

impl Lambda {
    pub fn new(body: LambdaBody, param_list: Parameters) -> Self {
        Self {
            body,
            param_list,
            captured: Vec::new(),
        }
    }
}
