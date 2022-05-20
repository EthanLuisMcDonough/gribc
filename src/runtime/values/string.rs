use super::HeapValue;
use ast::node::Program;
use runtime::memory::Gc;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};

#[derive(Clone, PartialEq, Eq)]
pub enum GribString {
    Stored(usize),
    Heap(usize),
    Char(char),
    Static(&'static str),
}

impl Default for GribString {
    fn default() -> Self {
        Self::Static("")
    }
}

impl GribString {
    pub fn as_ref<'a>(&self, program: &'a Program, gc: &'a Gc) -> Option<GribStringRef<'a>> {
        match self {
            GribString::Stored(ind) => program
                .strings
                .get(*ind)
                .map(|s| GribStringRef::Ref(s.as_str()))
                .into(),
            GribString::Heap(ind) => gc
                .heap_val(*ind)
                .and_then(|c| match c {
                    HeapValue::String(s) => Some(s.as_str()),
                    _ => None,
                })
                .map(GribStringRef::Ref),
            GribString::Static(r) => GribStringRef::Ref(r).into(),
            GribString::Char(c) => GribStringRef::Char(*c).into(),
        }
    }
}

pub enum GribStringRef<'a> {
    Ref(&'a str),
    Char(char),
}

impl Hash for GribStringRef<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Ref(s) => s.hash(state),
            Self::Char(c) => c.hash(state),
        }
    }
}

impl ToString for GribStringRef<'_> {
    fn to_string(&self) -> String {
        let mut st = String::new();
        match self {
            Self::Ref(s) => st.push_str(s),
            Self::Char(c) => st.push(*c),
        }
        st
    }
}

impl Default for GribStringRef<'_> {
    fn default() -> Self {
        Self::Ref("")
    }
}

impl<'a> From<GribStringRef<'a>> for Cow<'a, str> {
    fn from(s: GribStringRef<'_>) -> Cow<'_, str> {
        match s {
            GribStringRef::Char(c) => Cow::Owned(c.into()),
            GribStringRef::Ref(r) => Cow::Borrowed(r),
        }
    }
}

impl GribStringRef<'_> {
    pub fn cast_num(&self) -> Option<f64> {
        match self {
            Self::Ref(r) => r.parse().ok(),
            Self::Char(c) => c.to_digit(10).map(|c| c as f64),
        }
    }

    pub fn is_empty(&self) -> bool {
        if let Self::Ref(s) = self {
            s.is_empty()
        } else {
            false
        }
    }

    pub fn repeat(&self, amount: usize) -> String {
        match self {
            Self::Ref(r) => r.repeat(amount),
            Self::Char(c) => {
                let mut s = String::with_capacity(amount);
                for _ in 0..amount {
                    s.push(*c);
                }
                s
            }
        }
    }

    pub fn stringify(&self) -> String {
        match self {
            Self::Ref(r) => format!("{:?}", r),
            Self::Char(c) => format!("{:?}", c.to_string()),
        }
    }
}
