use ast::node::{CustomModule, GetProp, Import, Lambda, Procedure, Program, SetProp};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

struct ModuleData {
    module: CustomModule,
    index: usize,
}

pub struct Store {
    str_map: HashMap<String, usize>,
    mod_map: HashMap<PathBuf, ModuleData>,
    imports: Vec<Import>,
    functions: Vec<Procedure>,
    lambdas: Vec<Lambda>,
    getters: Vec<GetProp>,
    setters: Vec<SetProp>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            str_map: HashMap::new(),
            mod_map: HashMap::new(),
            imports: Vec::new(),
            functions: Vec::new(),
            lambdas: Vec::new(),
            getters: Vec::new(),
            setters: Vec::new(),
        }
    }

    pub fn ins_str(&mut self, s: String) -> usize {
        if let Some(ind) = self.str_map.get(&s) {
            return *ind;
        }

        let ind = self.str_map.len();
        self.str_map.insert(s, ind);

        ind
    }

    pub fn get_str(&self, key: &str) -> Option<&usize> {
        self.str_map.get(key)
    }

    pub fn ins_mod(&mut self, path: PathBuf, module: CustomModule) -> usize {
        let ind = self.mod_map.len();
        self.mod_map.insert(path, ModuleData { index: ind, module });
        ind
    }

    pub fn get_mod(&self, path: &Path) -> Option<&usize> {
        self.mod_map.get(path).map(|d| &d.index)
    }

    pub fn add_import(&mut self, import: Import) -> usize {
        let ind = self.imports.len();
        self.imports.push(import);
        ind
    }

    pub fn add_fn(&mut self, proc: Procedure) -> usize {
        let ind = self.functions.len();
        self.functions.push(proc);
        ind
    }

    pub fn add_lam(&mut self, lam: Lambda) -> usize {
        let ind = self.lambdas.len();
        self.lambdas.push(lam);
        ind
    }

    pub fn add_getter(&mut self, getter: GetProp) -> usize {
        let ind = self.getters.len();
        self.getters.push(getter);
        ind
    }

    pub fn add_setter(&mut self, setter: SetProp) -> usize {
        let ind = self.setters.len();
        self.setters.push(setter);
        ind
    }
}

impl From<Store> for Program {
    fn from(s: Store) -> Self {
        let mut p = Program::new();

        p.functions = s.functions;
        p.getters = s.getters;
        p.setters = s.setters;
        p.imports = s.imports;

        p.strings = vec![String::new(); s.str_map.len()];
        p.modules = vec![CustomModule::default(); s.mod_map.len()];

        for (string, index) in s.str_map {
            p.strings[index] = string;
        }

        for (_, data) in s.mod_map {
            p.modules[data.index] = data.module;
        }

        p
    }
}
