use std::collections::{HashMap, HashSet};

struct Capture {
    level: usize,
    identifiers: Vec<usize>,
}

pub struct CaptureStack {
    // Stack of captured stacks (for lambda metadata)
    stack: Vec<Capture>,
    // map of captured identifier names and the scopes in which they're declared
    markers: HashMap<usize, HashSet<usize>>,
}

impl CaptureStack {
    pub fn new() -> Self {
        Self {
            stack: vec![],
            markers: HashMap::new(),
        }
    }

    pub fn add(&mut self, level: usize) {
        self.stack.push(Capture {
            identifiers: Vec::new(),
            level,
        });
    }

    pub fn pop(&mut self) -> Vec<usize> {
        self.stack.pop().map(|e| e.identifiers).unwrap_or_default()
    }

    fn check_ref(&mut self, ident: usize, def: usize) {
        for Capture { level, identifiers } in &mut self.stack {
            if *level > def {
                identifiers.push(ident);
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
pub struct Scope {
    scope: HashMap<usize, DefData>,
    pub level: usize,
}

impl Scope {
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

    fn insert(&mut self, name: usize, kind: DefType) -> bool {
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

    pub fn insert_mut(&mut self, name: usize) -> bool {
        self.insert(name, DefType::Mutable)
    }
    pub fn insert_const(&mut self, name: usize) -> bool {
        self.insert(name, DefType::Constant)
    }
    pub fn insert_fn(&mut self, name: usize) -> bool {
        self.level == 0 && self.insert(name, DefType::Function)
    }
    pub fn insert_import(&mut self, name: usize) -> bool {
        self.insert(name, DefType::Import)
    }
    pub fn insert_var(&mut self, name: usize, is_mut: bool) -> bool {
        if is_mut {
            self.insert_mut(name)
        } else {
            self.insert_const(name)
        }
    }

    pub fn has(&self, name: usize, s: &mut CaptureStack) -> bool {
        if self.scope.contains_key(&name) {
            s.check_ref(name, self.level);
            return true;
        }
        false
    }
    pub fn has_editable(&self, name: usize, s: &mut CaptureStack) -> bool {
        if self
            .scope
            .get(&name)
            .filter(|d| d.kind == DefType::Mutable)
            .is_some()
        {
            s.check_ref(name, self.level);
            return true;
        }
        false
    }
}
