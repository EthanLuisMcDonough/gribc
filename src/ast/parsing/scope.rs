#[derive(Copy, Clone)]
pub struct Scope {
    pub in_loop: bool,
    pub in_proc: bool,
    pub in_lam: bool,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            in_loop: false,
            in_proc: false,
            in_lam: false,
        }
    }

    pub fn fn_proc() -> Self {
        Self {
            in_loop: false,
            in_proc: true,
            in_lam: false,
        }
    }

    pub fn fn_lam() -> Self {
        Self {
            in_loop: false,
            in_proc: true,
            in_lam: true,
        }
    }
    pub fn with_loop(mut self, in_loop: bool) -> Self {
        self.in_loop = in_loop;
        self
    }
}
