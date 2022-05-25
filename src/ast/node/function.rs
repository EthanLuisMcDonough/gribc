use super::{Block, Expression};
use location::Located;
use std::collections::HashSet;

pub type CaptureData = HashSet<String>;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Parameters {
    pub params: Vec<usize>,
    pub vardic: Option<usize>,
}

impl Parameters {
    pub fn new() -> Self {
        Parameters {
            params: Vec::new(),
            vardic: None,
        }
    }

    pub fn all_params(&'_ self) -> impl Iterator<Item = &'_ usize> {
        self.params.iter().chain(self.vardic.iter())
    }

    pub fn try_add(&mut self, name: usize) -> bool {
        if self.params.contains(&name) {
            false
        } else {
            self.params.push(name);
            true
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum LambdaBody {
    ImplicitReturn(Box<Expression>),
    Block(Block),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Procedure {
    pub identifier: Located<usize>,
    pub param_list: Parameters,
    pub body: Block,
    pub public: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Lambda {
    pub param_list: Parameters,
    pub body: LambdaBody,
    pub captured: HashSet<usize>,
}

impl Lambda {
    pub fn new(body: LambdaBody, param_list: Parameters) -> Self {
        Self {
            body,
            param_list,
            captured: HashSet::new(),
        }
    }
}

impl Default for Lambda {
    fn default() -> Self {
        Lambda {
            param_list: Parameters::new(),
            body: LambdaBody::Block(vec![]),
            captured: HashSet::new(),
        }
    }
}
