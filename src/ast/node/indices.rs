use super::{GetProp, Lambda, Procedure, SetProp, Import, CustomModule};
use typed_index_collections::TiVec;
use derive_more::{From, Into};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub struct ImportInd(usize);

/// Index pointing a string value stored inside the program AST node.
/// This node represents a string literal.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub struct StringVal(usize);

/// Index pointing a string value stored inside the program AST node.
/// This node represents an identifier.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub struct Identifier(usize);

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub struct ProcInd(usize);

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub struct LambdaInd(usize);

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub struct GetPropInd(usize);

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub struct SetPropInd(usize);

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub struct ModuleInd(usize);

impl From<Identifier> for StringVal {
    fn from(value: Identifier) -> Self {
        StringVal(value.0)
    }
}

pub type Imports = TiVec<ImportInd, Import>;
pub type Procedures = TiVec<ProcInd, Procedure>;
pub type Lambdas = TiVec<LambdaInd, Lambda>;
pub type Getters = TiVec<GetPropInd, GetProp>;
pub type Setters = TiVec<SetPropInd, SetProp>;
pub type StringStore = TiVec<StringVal, String>;
pub type ModuleStore = TiVec<ModuleInd, CustomModule>;
