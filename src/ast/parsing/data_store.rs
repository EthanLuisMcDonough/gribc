use ast::node::*;
use location::*;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use typed_index_collections::ti_vec;

type ModMap = HashMap<PathBuf, ModuleData>;
type StrMap = HashMap<String, StringVal>;

struct ModuleData {
    module: CustomModule,
    index: ModuleInd,
}

#[derive(Default)]
pub struct Store {
    str_map: StrMap,
    mod_map: ModMap,
    imports: Imports,
    procedures: Procedures,
    lambdas: Lambdas,
    getters: Getters,
    setters: Setters,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    fn gen_str_index<'a>(&mut self, s: impl Into<Cow<'a, str>>) -> usize {
        let s = s.into();
        if let Some(&ind) = self.str_map.get(s.as_ref()) {
            return ind.into();
        }

        let ind = self.str_map.len();
        self.str_map.insert(s.into_owned(), ind.into());
        ind
    }

    pub fn ins_str<'a>(&mut self, s: impl Into<Cow<'a, str>>) -> StringVal {
        self.gen_str_index(s).into()
    }

    pub fn ins_ident<'a>(&mut self, s: impl Into<Cow<'a, str>>) -> Identifier {
        self.gen_str_index(s).into()
    }

    pub fn get_str(&self, key: &str) -> Option<StringVal> {
        self.str_map.get(key).cloned()
    }

    pub fn get_ident(&self, key: &str) -> Option<Identifier> {
        self.str_map
            .get(key)
            .cloned()
            // We shouldn't do this a lot, but this is one of the places
            // where we can
            .map(|str_val| Identifier::from(usize::from(str_val)))
    }

    pub fn ins_mod(&mut self, path: PathBuf, module: CustomModule) -> ModuleInd {
        let ind = self.mod_map.len();
        self.mod_map.insert(
            path,
            ModuleData {
                index: ind.into(),
                module,
            },
        );
        ind.into()
    }

    pub fn get_mod(&self, path: &Path) -> Option<ModuleInd> {
        self.mod_map.get(path).map(|d| d.index.into())
    }

    pub fn add_import(&mut self, import: Import) -> ImportInd {
        let ind = self.imports.len();
        self.imports.push(import);
        ind.into()
    }

    pub fn add_proc(&mut self, proc: Procedure) -> ProcInd {
        let ind = self.procedures.len();
        self.procedures.push(proc);
        ind.into()
    }

    pub fn add_lam(&mut self, lam: Lambda) -> LambdaInd {
        let ind = self.lambdas.len();
        self.lambdas.push(lam);
        ind.into()
    }

    pub fn add_getter(&mut self, getter: GetProp) -> GetPropInd {
        let ind = self.getters.len();
        self.getters.push(getter);
        ind.into()
    }

    pub fn add_setter(&mut self, setter: SetProp) -> SetPropInd {
        let ind = self.setters.len();
        self.setters.push(setter);
        ind.into()
    }
}

// Change imports so that "all" imports have a list of string indexes
// of all identifiers that are actually used in the program
// This is specifically for native modules
fn rewrite_imports(imports: &mut Imports, str_map: &StrMap) {
    for import in imports {
        if let Import {
            module:
                Located {
                    data: Module::Native(package),
                    ..
                },
            kind: ImportKind::All,
        } = import
        {
            let items = package
                .raw_names()
                .iter()
                .filter_map(|n| str_map.get(*n))
                .map(|&data| Located {
                    // "cast" value from string to identifier
                    data: Identifier::from(usize::from(data)),
                    start: Location::default(),
                    end: Location::default(),
                })
                .collect();
            import.kind = ImportKind::List(items);
        }
    }
}

impl From<Store> for Program {
    fn from(s: Store) -> Self {
        let mut p = Program::new();

        p.procedures = s.procedures;
        p.getters = s.getters;
        p.setters = s.setters;
        p.imports = s.imports;
        p.lambdas = s.lambdas;

        p.modules = ti_vec![CustomModule::default(); s.mod_map.len()];

        rewrite_imports(&mut p.imports, &s.str_map);

        for (_, mut data) in s.mod_map {
            rewrite_imports(&mut data.module.imports, &s.str_map);
            p.modules[data.index] = data.module;
        }

        p.strings = ti_vec![String::new(); s.str_map.len()];
        for (string, index) in s.str_map {
            p.strings[index] = string;
        }

        for module in &mut p.modules {
            for (ind, procedure) in module.procedures.iter_enumerated() {
                let ident_str: StringVal = procedure.identifier.data.into();
                let key = p.strings[ident_str].clone().into();
                module.lookup.insert(key, ind);
            }
        }

        p
    }
}
