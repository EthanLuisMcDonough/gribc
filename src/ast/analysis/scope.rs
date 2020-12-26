use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
enum DefType {
    Mutable,
    Constant,
    Function,
}

#[derive(Clone, Copy, PartialEq)]
struct DefData {
    kind: DefType,
    level: usize,
}

#[derive(Clone, PartialEq)]
pub struct Scope<'a> {
    scope: HashMap<&'a str, DefData>,
    level: usize,
}

impl<'a> Scope<'a> {
    pub fn new() -> Self {
        Self {
            scope: HashMap::new(),
            level: 0
        }
    }
    pub fn sub(&self) -> Self {
        Self {
            scope: self.scope.clone(),
            level: self.level + 1,
        }
    }
    pub fn proc_scope(&self) -> Self {
        Self {
            scope: self.scope.clone().into_iter()
                .filter(|(_, DefData { kind, .. })| *kind == DefType::Function)
                .collect(),
            level: self.level + 1
        }
    }
    fn insert(&mut self, name: &'a str, kind: DefType) -> bool {
        self.scope.insert(name, DefData {
            level: self.level, kind,
        })
        .filter(|d| d.level == self.level)
        .is_none()
    }
    pub fn insert_mut(&mut self, name: &'a str) -> bool {
        self.insert(name, DefType::Mutable)
    }
    pub fn insert_const(&mut self, name: &'a str) -> bool {
        self.insert(name, DefType::Constant)
    }
    pub fn insert_fn(&mut self, name: &'a str) -> bool {
        self.level == 0 && self.insert(name, DefType::Function)
    }
    pub fn insert_var(&mut self, name: &'a str, is_mut: bool) -> bool {
        if is_mut { self.insert_mut(name) } else { self.insert_const(name) }
    }
    pub fn has(&self, name: &'a str) -> bool {
        self.scope.contains_key(name)
    }
    pub fn has_editable(&self, name: &'a str) -> bool {
        self.scope.get(name).filter(|d| d.kind == DefType::Mutable).is_some()
    }
}
