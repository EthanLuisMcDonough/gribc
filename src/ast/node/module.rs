use super::{Import, NativePackage, Procedure, Program};
use runtime::values::Callable;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Module {
    Custom(usize),
    Native(NativePackage),
}

impl Module {
    fn custom_index(&self) -> Option<usize> {
        match self {
            Self::Custom(ind) => Some(*ind),
            _ => None,
        }
    }

    pub fn names<'a>(&self, program: &'a Program) -> HashSet<&'a str> {
        match self {
            Module::Custom(ind) => program.modules[*ind].get_functions(),
            Module::Native(n) => n.raw_names().iter().map(|f| *f).collect(),
        }
    }

    pub fn iter<'a>(&'a self, program: &Program) -> ModFns<'a> {
        ModFns {
            index: 0,
            module: self,
            fnc_max: self
                .custom_index()
                .and_then(|i| program.modules.get(i))
                .map(|mods| mods.functions.len())
                .unwrap_or(0),
        }
    }
}

pub struct ModFns<'a> {
    index: usize,
    module: &'a Module,
    fnc_max: usize,
}

impl<'a> Iterator for ModFns<'a> {
    type Item = Callable;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        match self.module {
            Module::Custom(i) if i < &self.fnc_max => Some(Callable::Procedure {
                index,
                module: Some(*i),
            }),
            Module::Native(n) => n
                .raw_names()
                .get(index)
                .and_then(|s| n.fn_from_str(s))
                .map(Callable::Native),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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
    pub fn get_functions<'a>(&'a self) -> HashSet<&'a str> {
        self.functions
            .iter()
            .filter(|f| f.public)
            .map(|f| f.identifier.data.as_str())
            .collect()
    }

    pub fn has_function(&self, name: &str) -> bool {
        self.functions
            .iter()
            .filter(|f| f.public)
            .any(|f| f.identifier.data == name)
    }
}
