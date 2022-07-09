use super::{Import, NativePackage, Procedure, Program};
use runtime::values::Callable;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Module {
    Custom(usize),
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
    pub imports: Vec<Import>,
    pub functions: Vec<Procedure>,
    pub path: PathBuf,
    pub lookup: HashMap<Cow<'static, str>, usize>,
}

impl CustomModule {
    /// Iterator of module's public functions
    pub fn pub_functions(&self) -> impl Iterator<Item = &Procedure> {
        self.functions.iter().filter(|f| f.public)
    }

    /// Returns the index of the public function with the given index-name
    pub fn get_function(&self, name: usize) -> Option<usize> {
        self.functions
            .iter()
            .filter(|f| f.public)
            .position(|f| f.identifier.data == name)
    }
}
