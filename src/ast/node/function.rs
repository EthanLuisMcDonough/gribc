use super::{Block, Expression};
use location::Located;
use std::collections::HashSet;

pub type CaptureData = HashSet<String>;

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
pub enum LambdaBody {
    ImplicitReturn(Box<Expression>),
    Block(Block),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Procedure {
    pub identifier: Located<String>,
    pub param_list: Parameters,
    pub body: Block,
    pub public: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Lambda {
    pub param_list: Parameters,
    pub body: LambdaBody,
    pub captured: HashSet<String>,
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
