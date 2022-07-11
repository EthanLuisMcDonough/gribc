use runtime::{
    memory::{Gc, StackSlot},
    values::GribValue,
};

#[derive(Clone)]
enum LamState {
    None,
    Inside(Option<usize>),
}

#[derive(Clone)]
pub struct LocalState {
    this: Option<GribValue>,
    lambda: LamState,
}

impl Default for LocalState {
    fn default() -> Self {
        Self {
            this: None,
            lambda: LamState::None,
        }
    }
}

impl LocalState {
    pub fn new(this: impl Into<Option<GribValue>>, stack: Option<usize>) -> Self {
        Self {
            this: this.into(),
            lambda: LamState::Inside(stack),
        }
    }

    pub fn in_lam(&self) -> bool {
        if let LamState::Inside(_) = self.lambda {
            true
        } else {
            false
        }
    }

    pub fn get_stack(&self) -> Option<usize> {
        if let LamState::Inside(ind) = self.lambda {
            ind.clone()
        } else {
            None
        }
    }

    pub fn get_this(&self) -> GribValue {
        self.this.clone().unwrap_or_default()
    }

    pub fn with_this(&mut self, this: GribValue) {
        self.this = Some(this);
    }

    pub fn with_lam(&mut self) {
        self.lambda = LamState::Inside(None);
    }

    pub fn with_stack(&mut self, stack: Option<usize>) {
        self.lambda = LamState::Inside(stack);
    }

    pub fn stack_item<'a>(&self, index: usize, gc: &'a Gc) -> Option<&'a StackSlot> {
        self.get_stack()
            .and_then(|ind| gc.try_get_stack(ind).and_then(|stack| stack.get(index)))
    }
}
