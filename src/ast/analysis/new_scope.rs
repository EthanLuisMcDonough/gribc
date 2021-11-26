use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

struct Capture {
    level: usize,
    captures: Vec<usize>,
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
            if *level > def {
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
    stack_pos: usize,
    level: usize,
}

/*use std::collections::HashSet;
use std::rc::Rc;

fn main() {
    let s = Rc::new(String::from("thingy"));
    let mut set = HashSet::new();
    set.insert(s);
    println!("{}", set.contains(&String::from("thingy")));

    println!("Hello, world!");
}*/

pub struct GlobalScope {
    scope: HashMap<Rc<String>, Vec<DefData>>,
    stack: Vec<Rc<String>>,
}

pub struct Scope {
    level: usize,
    allocations: usize,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            level: 0,
            allocations: 0,
        }
    }

    pub fn sub(&self) -> Self {
        Self {
            level: self.level + 1,
            allocations: 0,
        }
    }

    // true if entry was inserted
    fn insert(&mut self, g: &mut GlobalScope, label: String, kind: DefType) -> bool {
        let label = Rc::new(label);
        let instances = g.scope.entry(label.clone()).or_insert(vec![]);

        let data = DefData {
            kind,
            stack_pos: g.stack.len(),
            level: self.level,
        };

        if instances
            .last()
            .filter(|d| d.level == self.level && d.kind != DefType::Import)
            .is_none()
        {
            instances.push(data);
            g.stack.push(label);
            self.allocations += 1;
            return true;
        }

        false
    }

    pub fn insert_mut(&mut self, g: &mut GlobalScope, name: String) -> bool {
        self.insert(g, name, DefType::Mutable)
    }
    pub fn insert_const(&mut self, g: &mut GlobalScope, name: String) -> bool {
        self.insert(g, name, DefType::Constant)
    }
    pub fn insert_fn(&mut self, g: &mut GlobalScope, name: String) -> bool {
        self.level == 0 && self.insert(g, name, DefType::Function)
    }
    pub fn insert_import(&mut self, g: &mut GlobalScope, name: String) -> bool {
        self.insert(g, name, DefType::Import)
    }
    pub fn insert_var(&mut self, g: &mut GlobalScope, name: String, is_mut: bool) -> bool {
        if is_mut {
            self.insert_mut(g, name)
        } else {
            self.insert_const(g, name)
        }
    }

    pub fn clean(self, g: &mut GlobalScope) {
        let ind = g
            .stack
            .len()
            .checked_sub(1 + self.allocations)
            .unwrap_or_default();
        for ident in g.stack.drain(ind..) {
            g.scope[&ident].pop();
        }
    }
}

/*#[derive(PartialEq, Clone)]
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
*/
