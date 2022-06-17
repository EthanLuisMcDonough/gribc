use super::{Import, NativePackage, Procedure, Program};
use runtime::values::Callable;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Module {
    Custom(usize),
    Native {
        package: NativePackage,
        indices: HashSet<usize>,
    },
}

impl Module {
    fn custom_index(&self) -> Option<usize> {
        match self {
            Self::Custom(ind) => Some(*ind),
            _ => None,
        }
    }

    pub fn is_native(&self) -> bool {
        if let Self::Native { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn names(&self, program: &Program) -> HashSet<usize> {
        match self {
            Module::Custom(ind) => program.modules[*ind].get_functions(),
            Module::Native { indices, .. } => indices.clone(),
        }
    }

    pub fn callables(&self, program: &Program) -> Vec<(Callable, usize)> {
        match self {
            Module::Custom(i) => program.modules[*i]
                .functions
                .iter()
                .filter(|f| f.public)
                .enumerate()
                .map(|(index, fnc)| {
                    (
                        Callable::Procedure {
                            index,
                            module: Some(*i),
                        },
                        fnc.identifier.data,
                    )
                })
                .collect(),
            Module::Native { package, indices } => indices
                .iter()
                .flat_map(|ind| {
                    program
                        .strings
                        .get(*ind)
                        .and_then(|s| package.fn_from_str(s))
                        .map(Callable::Native)
                        .map(|c| (c, *ind))
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CustomModule {
    pub imports: Vec<Import>,
    pub functions: Vec<Procedure>,
    pub path: PathBuf,
}

impl Default for CustomModule {
    fn default() -> Self {
        Self {
            imports: Vec::new(),
            functions: Vec::new(),
            path: PathBuf::default(),
        }
    }
}

impl CustomModule {
    pub fn get_functions(&self) -> HashSet<usize> {
        self.functions
            .iter()
            .filter(|f| f.public)
            .map(|f| f.identifier.data)
            .collect()
    }

    pub fn has_function(&self, name: usize) -> bool {
        self.functions
            .iter()
            .filter(|f| f.public)
            .any(|f| f.identifier.data == name)
    }
}
