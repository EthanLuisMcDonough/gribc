use super::{NativePackage, Procedures, Imports, Program, Identifier, ModuleInd, ProcInd};
use runtime::values::Callable;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Module {
    Custom(ModuleInd),
    Native(NativePackage),
}

impl Module {
    pub fn is_native(&self) -> bool {
        if let Self::Native { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn get_callable(&self, string: &str, program: &Program) -> Option<Callable> {
        match self {
            Self::Native(native) => native.fn_from_str(&string).map(Callable::Native),
            Self::Custom(mod_ind) => {
                program.modules[*mod_ind]
                    .lookup
                    .get(string)
                    .map(|&fn_ind| Callable::Procedure {
                        module: Some(*mod_ind),
                        index: fn_ind,
                    })
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct CustomModule {
    pub imports: Imports,
    pub procedures: Procedures,
    pub path: PathBuf,
    pub lookup: HashMap<Cow<'static, str>, ProcInd>,
}

impl CustomModule {
    pub fn new(path: PathBuf) -> Self {
        Self { path, ..Default::default() }
    }

    /// Returns the index of the public function with the given index-name
    pub fn get_procedure(&self, name: Identifier) -> Option<ProcInd> {
        for (ind, proc) in self.procedures.iter_enumerated() {
            if proc.identifier.data == name {
                return Some(ind).filter(|_| proc.public);
            }
        }
        None
    }
}
