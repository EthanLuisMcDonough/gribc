use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

struct Capture {
    level: usize,
    identifiers: HashSet<String>,
}

pub struct CaptureStack {
    stack: Vec<Capture>,
}

impl CaptureStack {
    pub fn new() -> Self {
        Self { stack: vec![] }
    }

    pub fn add(&mut self, level: usize) {
        self.stack.push(Capture {
            identifiers: HashSet::new(),
            level,
        });
    }

    pub fn pop(&mut self) -> HashSet<String> {
        self.stack.pop().map(|e| e.identifiers).unwrap_or_default()
    }

    fn check_ref(&mut self, s: &str, def: usize) {
        for Capture { level, identifiers } in &mut self.stack {
            if *level >= def {
                identifiers.insert(s.to_owned());
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum DefType {
    Mutable,
    Constant,
    Function,
    Import,
}

#[derive(Clone, Copy, PartialEq)]
struct DefData {
    kind: DefType,
    level: usize,
}

#[derive(PartialEq, Clone)]
pub struct Scope<'a> {
    scope: HashMap<&'a str, DefData>,
    pub level: usize,
}

impl<'a> Scope<'a> {
    pub fn new() -> Self {
        Self {
            scope: HashMap::new(),
            level: 0,
        }
    }
    pub fn sub(&self) -> Self {
        Self {
            scope: self.scope.clone(),
            level: self.level + 1,
        }
    }

    fn insert(&mut self, name: &'a str, kind: DefType) -> bool {
        self.scope
            .insert(
                name,
                DefData {
                    level: self.level,
                    kind,
                },
            )
            .filter(|d| d.level == self.level && d.kind != DefType::Import)
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
    pub fn insert_import(&mut self, name: &'a str) -> bool {
        self.insert(name, DefType::Import)
    }
    pub fn insert_var(&mut self, name: &'a str, is_mut: bool) -> bool {
        if is_mut {
            self.insert_mut(name)
        } else {
            self.insert_const(name)
        }
    }

    pub fn has(&self, name: &str, s: &mut CaptureStack) -> bool {
        if self.scope.contains_key(name) {
            s.check_ref(name, self.level);
            return true;
        }
        false
    }
    pub fn has_editable(&self, name: &str, s: &mut CaptureStack) -> bool {
        if self
            .scope
            .get(name)
            .filter(|d| d.kind == DefType::Mutable)
            .is_some()
        {
            s.check_ref(name, self.level);
            return true;
        }
        false
    }
}
