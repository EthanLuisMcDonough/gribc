use super::WalkResult;
use ast::node::*;
use runtime::values::Callable;
use std::collections::HashMap;

/// A lambda analysis's state
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LamPass {
    First,
    Second,
}

/// Marks the transition of a scope
pub enum SubState {
    /// Only resets local declaration counter
    NoChange,
    /// Sets the local loop counter to an active zero value
    WithLoop,
    /// Sets loop counter to None and function counter
    /// to an active zero
    InFunc,
}

#[derive(Debug)]
struct Capture {
    level: usize,
    identifiers: Vec<usize>,
}

impl Capture {
    fn new(level: usize) -> Self {
        Self {
            level,
            identifiers: Vec::new(),
        }
    }

    fn insert(&mut self, name: usize) {
        if !self.identifiers.contains(&name) {
            self.identifiers.push(name);
        }
    }
}

/// Captured stack associated with a particular lambda or getter/setter
#[derive(Debug)]
pub struct CaptureStack {
    stack: Vec<Capture>,
}

impl CaptureStack {
    pub fn new() -> Self {
        Self { stack: vec![] }
    }

    pub fn add(&mut self, level: usize) {
        self.stack.push(Capture::new(level));
    }

    /// Pops off the top captured stack and converts the array of identifiers
    /// to an array of index offsets.  The scope passed in must be a copy of
    /// self before the analysis took place.
    pub fn pop(&mut self, top_scope: &mut Scope) -> Vec<usize> {
        self.stack
            .pop()
            .map(|end| {
                let (names, captured): (Vec<usize>, _) = end
                    .identifiers
                    .into_iter()
                    // Filter out any variables that aren't valid captures
                    .filter_map(|name| {
                        let val = top_scope.runtime_value(name);
                        if let Some(RuntimeValue::StackOffset(off)) = val {
                            Some((name, off))
                        } else {
                            // This area shouldn't be reachable
                            None
                        }
                    })
                    .unzip();

                // We can insert all the valid names as mutable because we've already checked
                // for mutability errors in the first lambda pass
                // This second pass serves only to calculate the stack offsets
                for name in names {
                    top_scope.insert_mut(name);
                }

                captured
            })
            .unwrap_or_default()
    }

    /// Returns true if any captured lambda scopes were edited during a variable check
    /// If so, the checked variable should be marked as a captured variable
    fn check_ref(&mut self, ident: usize, current: usize) -> bool {
        let mut changed = false;
        for capture in &mut self.stack {
            if capture.level > current {
                capture.insert(ident);
                changed = true;
            }
        }
        changed
    }
}

#[derive(Clone, PartialEq, Debug)]
enum DefType {
    Mutable { captured: bool, stack_pos: usize },
    Constant { stack_pos: usize },
    Function(Callable),
    Import(StaticValue),
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

#[derive(PartialEq, Debug, Default, Clone)]
pub struct Scope {
    /// The variable hashmap
    scope: HashMap<usize, DefData>,
    /// The scope depth
    pub level: usize,
    /// The number of items on the stack
    pub stack: usize,
    /// The number of declarations in the block
    pub local: usize,
    /// The number of declarations in the current loop or function
    /// Functions and loops are responsible for cleaning up their own
    /// loop declartions, parameters, and variables transferred from
    /// a captured stack
    pub loop_alloc: Option<usize>,
    pub fnc_alloc: Option<usize>,
    /// Whether the scope is in a lambda and what analysis pass it is on
    pub lam_pass: Option<LamPass>,
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if we're not in a lambda or if we're in the first pass
    /// Basically tells us if the first half of the analysis can take place
    pub fn in_first_pass(&self) -> bool {
        self.lam_pass
            .as_ref()
            .filter(|p| **p != LamPass::First)
            .is_none()
    }

    /// Same as previous but for second pass
    pub fn in_second_pass(&self) -> bool {
        self.lam_pass
            .as_ref()
            .filter(|p| **p != LamPass::Second)
            .is_none()
    }

    pub fn sub_with<F: FnOnce(&mut Self) -> WalkResult>(
        &mut self,
        state: SubState,
        fnc: F,
    ) -> WalkResult {
        let mut new_scope = self.clone();
        new_scope.level += 1;
        new_scope.local = 0;

        match state {
            SubState::NoChange => {}
            SubState::InFunc => {
                new_scope.fnc_alloc = Some(0);
                new_scope.loop_alloc = None;
            }
            SubState::WithLoop => {
                new_scope.loop_alloc = Some(0);
            }
        }

        fnc(&mut new_scope)?;
        new_scope.migrate(self);

        Ok(())
    }

    pub fn sub<F: FnOnce(&mut Self) -> WalkResult>(&mut self, fnc: F) -> WalkResult {
        self.sub_with(SubState::NoChange, fnc)
    }

    pub fn sub_loop<F: FnOnce(&mut Self, &mut Block) -> WalkResult>(
        &mut self,
        fnc: F,
        block: &mut Block,
    ) -> WalkResult {
        self.sub_with(SubState::WithLoop, |scope| {
            fnc(scope, block)?;
            scope.check_decls(block);
            Ok(())
        })
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
        self.sub_with(SubState::InFunc, |scope| {
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

    pub fn add_params(&mut self, params: &Parameters) {
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
        let stack_pos = self.new_alloc();
        self.insert(
            name,
            DefType::Mutable {
                captured: false,
                stack_pos,
            },
        )
    }

    fn new_decl(&mut self) {
        self.local += 1;

        if let Some(loop_count) = &mut self.loop_alloc {
            *loop_count += 1;
        }

        if let Some(fnc_count) = &mut self.fnc_alloc {
            *fnc_count += 1;
        }
    }

    fn new_alloc(&mut self) -> usize {
        let pos = self.stack;
        self.stack += 1;
        pos
    }

    pub fn insert_const(&mut self, name: usize) -> bool {
        let stack_pos = self.new_alloc();
        self.insert(name, DefType::Constant { stack_pos })
    }

    pub fn insert_fn(&mut self, name: usize, index: usize, module: Option<usize>) -> bool {
        self.level == 0
            && self.insert(
                name,
                DefType::Function(Callable::Procedure { index, module }),
            )
    }

    pub fn import_module(&mut self, name: usize, module: Module) -> bool {
        self.insert(name, DefType::Import(StaticValue::Module(module)))
    }

    pub fn import_function(&mut self, name: usize, module: usize, index: usize) -> bool {
        self.insert(
            name,
            DefType::Import(StaticValue::Function(Callable::Procedure {
                module: Some(module),
                index,
            })),
        )
    }

    pub fn native_function(&mut self, name: usize, fnc: NativeFunction) {
        if self.level == 0 {
            self.insert(
                name,
                DefType::Import(StaticValue::Function(Callable::Native(fnc))),
            );
        }
    }

    pub fn insert_var(&mut self, name: usize, is_mut: bool) -> bool {
        self.new_decl();
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

    /// Gets the runtime value of a variable (stack offset, raw funtion, or module object)
    pub fn runtime_value(&self, name: usize) -> Option<RuntimeValue> {
        self.scope.get(&name).map(|val| match &val.kind {
            DefType::Import(value) => RuntimeValue::Static(value.clone()),
            DefType::Function(fnc) => RuntimeValue::Static(StaticValue::Function(fnc.clone())),
            DefType::Constant { stack_pos } | DefType::Mutable { stack_pos, .. } => {
                RuntimeValue::StackOffset(self.stack - stack_pos)
            }
        })
    }

    /// Checks if an identifier in an expression exists
    /// If it does and is out of scope, it will try to add it to any active lambda captures
    pub fn has(&mut self, name: usize, s: &mut CaptureStack) -> bool {
        if let Some(data) = self.scope.get_mut(&name) {
            if s.check_ref(name, data.level) {
                data.try_capture();
            }
            return true;
        }
        false
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
