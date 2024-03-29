use super::Scope;
use ast::node::RuntimeValue;

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

/// A vec of captures is associated with a particular lambda or getter/setter
/// Whenever a lambda/getter/setter is being walked, a new captured stack
/// is created.  Any variables ouside the lambda currently being walked
/// are added to each unimplemented!()
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
    pub(super) fn check_ref(&mut self, ident: usize, current: usize) -> bool {
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
