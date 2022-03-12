use super::{Import, NativePackage, Procedure, Program};
use runtime::values::Callable;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Module {
    Custom(usize),
    Native {
        package: NativePackage,
        indices: Vec<usize>,
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
            Module::Native(native) => native.indices.clone().into_iter().collect(),
        }
    }

    pub fn iter<'a>(&'a self, program: &'a Program) -> ModFns<'a> {
        ModFns {
            index: 0,
            module: self,
            program,
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
    program: &'a Program,
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
            Module::Native { package, indices } => indices
                .get(index)
                .and_then(|ind| self.program.strings.get(*ind))
                .and_then(|s| package.fn_from_str(s))
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
