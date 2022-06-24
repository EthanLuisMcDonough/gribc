use super::WalkResult;
use ast::node::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
struct Capture {
    level: usize,
    identifiers: HashSet<usize>,
}

#[derive(Debug)]
pub struct CaptureStack {
    // Stack of captured stacks (for lambda metadata)
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

    pub fn pop(&mut self) -> HashSet<usize> {
        self.stack.pop().map(|e| e.identifiers).unwrap_or_default()
    }

    /// Returns true if any captured scopes were edited
    fn check_ref(&mut self, ident: usize, current: usize) -> bool {
        let mut changed = false;
        for Capture { level, identifiers } in &mut self.stack {
            if *level > current {
                identifiers.insert(ident);
                changed = true;
            }
        }
        changed
    }
}

#[derive(Clone, PartialEq, Debug)]
enum ScopeImport {
    Module(Module),
    Function { module: usize, index: usize },
    NativeFn(NativeFunction),
}

#[derive(Clone, PartialEq, Debug)]
enum DefType {
    Mutable { captured: bool },
    Constant,
    Function { module: Option<usize>, index: usize },
    Import(ScopeImport),
}

#[derive(Clone, PartialEq, Debug)]
struct DefData {
    kind: DefType,
    level: usize,
}

impl DefData {
    fn is_mut(&self) -> bool {
        if let DefType::Mutable { .. } = self.kind {
            true
        } else {
            false
        }
    }

    fn is_captured(&self) -> bool {
        if let DefType::Mutable { captured, .. } = self.kind {
            captured
        } else {
            false
        }
    }

    fn try_capture(&mut self) {
        if let DefType::Mutable { captured, .. } = &mut self.kind {
            *captured = true;
        }
    }

    fn is_import(&self) -> bool {
        if let DefType::Import { .. } = self.kind {
            true
        } else {
            false
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct Scope {
    scope: HashMap<usize, DefData>,
    pub level: usize,
}

impl Scope {
    pub fn new() -> Scope {
        Self {
            scope: HashMap::new(),
            level: 0,
        }
    }

    pub fn sub<F: FnOnce(&mut Self) -> WalkResult>(&mut self, fnc: F) -> WalkResult {
        let mut new_scope = Self {
            scope: self.scope.clone(),
            level: self.level + 1,
        };

        fnc(&mut new_scope)?;
        new_scope.migrate(self);

        Ok(())
    }

    pub fn sub_block<F: FnOnce(&mut Self, &mut Block) -> WalkResult>(
        &mut self,
        fnc: F,
        block: &mut Block,
    ) -> WalkResult {
        self.sub(|scope| {
            fnc(scope, block)?;
            scope.check_decls(block);
            Ok(())
        })
    }

    pub fn sub_params<F: FnOnce(&mut Self, &mut Parameters) -> WalkResult>(
        &mut self,
        fnc: F,
        params: &mut Parameters,
    ) -> WalkResult {
        self.sub(|scope| {
            scope.add_params(params);
            fnc(scope, params)?;
            scope.check_params(params);
            Ok(())
        })
    }

    pub fn sub_fnc<F: FnOnce(&mut Self, &mut Parameters, &mut Block) -> WalkResult>(
        &mut self,
        fnc: F,
        params: &mut Parameters,
        block: &mut Block,
    ) -> WalkResult {
        self.sub(|scope| {
            scope.add_params(params);
            fnc(scope, params, block)?;
            scope.check_params(params);
            scope.check_decls(block);
            Ok(())
        })
    }

    pub fn check_params(&mut self, params: &mut Parameters) {
        for param in params.all_params_mut() {
            if self
                .scope
                .remove(&param.name)
                .filter(|d| d.is_captured())
                .is_some()
            {
                param.captured = true;
            }
        }
    }

    pub fn check_decls(&mut self, block: &mut Block) {
        for stmt in block.iter_mut() {
            if let Node::Declaration(Declaration {
                mutable: true,
                declarations,
            })
            | Node::For {
                declaration:
                    Some(Declaration {
                        mutable: true,
                        declarations,
                    }),
                ..
            } = stmt
            {
                for decl in declarations.iter_mut() {
                    let name = decl.identifier.data;
                    if self
                        .scope
                        .remove(&name)
                        .filter(|d| d.is_captured())
                        .is_some()
                    {
                        decl.captured = true;
                    }
                }
            }
        }
    }

    fn add_params(&mut self, params: &Parameters) {
        for param in params.all_params() {
            self.insert_mut(param.name);
        }
    }

    fn migrate(self, parent: &mut Self) {
        for (name, data) in self.scope {
            if data.is_captured() {
                parent.try_capture(name);
            }
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
            .filter(|d| d.level == self.level && !d.is_import())
            .is_none()
    }

    pub fn insert_mut(&mut self, name: usize) -> bool {
        self.insert(name, DefType::Mutable { captured: false })
    }

    pub fn insert_const(&mut self, name: usize) -> bool {
        self.insert(name, DefType::Constant)
    }

    pub fn insert_fn(&mut self, name: usize, index: usize, module: Option<usize>) -> bool {
        self.level == 0 && self.insert(name, DefType::Function { index, module })
    }

    pub fn import_module(&mut self, name: usize, module: Module) -> bool {
        self.insert(name, DefType::Import(ScopeImport::Module(module)))
    }

    pub fn import_function(&mut self, name: usize, module: usize, index: usize) -> bool {
        self.insert(
            name,
            DefType::Import(ScopeImport::Function { module, index }),
        )
    }

    pub fn native_function(&mut self, name: usize, fnc: NativeFunction) {
        if self.level == 0 {
            self.insert(name, DefType::Import(ScopeImport::NativeFn(fnc)));
        }
    }

    pub fn insert_var(&mut self, name: usize, is_mut: bool) -> bool {
        if is_mut {
            self.insert_mut(name)
        } else {
            self.insert_const(name)
        }
    }

    pub fn is_captured(&self, name: usize) -> bool {
        self.scope.get(&name).filter(|d| d.is_captured()).is_some()
    }

    fn try_capture(&mut self, name: usize) {
        if let Some(data) = self.scope.get_mut(&name) {
            data.try_capture();
        }
    }

    pub fn prop_check(&mut self, name: usize) -> bool {
        if let Some(data) = self.scope.get_mut(&name) {
            data.try_capture();
            return true;
        }
        false
    }

    pub fn prop_check_mut(&mut self, name: usize) -> bool {
        if let Some(data) = self.scope.get_mut(&name).filter(|d| d.is_mut()) {
            data.try_capture();
            return true;
        }
        false
    }

    pub fn has(&mut self, name: usize, s: &mut CaptureStack) -> bool {
        if let Some(data) = self.scope.get_mut(&name) {
            if s.check_ref(name, data.level) {
                data.try_capture();
            }
            return true;
        }
        false
    }

    pub fn try_static(&self, name: usize) -> Option<ImportValue> {
        match self.scope.get(&name) {
            Some(DefData {
                kind: DefType::Import(value),
                ..
            }) => Some(match value {
                ScopeImport::Function { module, index } => ImportValue::Function {
                    module: Some(*module),
                    index: *index,
                },
                ScopeImport::NativeFn(native) => ImportValue::NativeFunction(native.clone()),
                ScopeImport::Module(module) => ImportValue::Module(module.clone()),
            }),
            Some(DefData {
                kind: DefType::Function { module, index },
                ..
            }) => Some(ImportValue::Function {
                module: module.clone(),
                index: *index,
            }),
            _ => None,
        }
    }

    pub fn has_editable(&mut self, name: usize, s: &mut CaptureStack) -> bool {
        if let Some(data) = self.scope.get_mut(&name).filter(|d| d.is_mut()) {
            if s.check_ref(name, data.level) {
                data.try_capture();
            }
            return true;
        }
        false
    }
}
