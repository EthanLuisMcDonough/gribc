use super::{Block, Expression};
use location::Located;
use std::collections::HashSet;

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
    //pub captured: HashSet<String>,
}
