use std::path::Path;

#[derive(Copy, Clone)]
pub struct Scope<'a> {
    pub in_loop: bool,
    pub in_proc: bool,
    pub is_top: bool,
    pub path: Option<&'a Path>,
}

impl<'a> Scope<'a> {
    pub fn new(path: &'a Path) -> Self {
        let path = Some(path);
        Self {
            in_loop: false,
            in_proc: false,
            is_top: true,
            path,
        }
    }

    pub fn fn_sub() -> Self {
        Self {
            in_loop: false,
            in_proc: true,
            is_top: false,
            path: None,
        }
    }
    
    pub fn with_loop(mut self, in_loop: bool) -> Self {
        self.in_loop = in_loop;
        self
    }

    pub fn with_proc(mut self, in_proc: bool) -> Self {
        self.in_proc = in_proc;
        self
    }

    pub fn next_level(mut self) -> Self {
        self.is_top = false;
        self
    }

    pub fn with_path(mut self, path: &'a Path) -> Self {
        self.path = Some(path);
        self
    }
}