use super::HeapValue;
use ast::node::Program;
use runtime::memory::Gc;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

#[derive(Clone, PartialEq, Eq, Debug)]
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
            Self::Char(c) => {
                let mut bytes = [0u8; 4];
                c.encode_utf8(&mut bytes).hash(state)
            }
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

impl PartialEq for GribStringRef<'_> {
    fn eq(&self, other: &GribStringRef) -> bool {
        use self::GribStringRef::*;
        match (self, other) {
            (Ref(r1), Ref(r2)) => r1 == r2,
            (Char(c1), Char(c2)) => c1 == c2,
            (Char(c), Ref(r)) | (Ref(r), Char(c)) => {
                let mut bytes = [0u8; 4];
                let char_ref = c.encode_utf8(&mut bytes);
                char_ref == *r
            }
        }
    }
}

impl PartialOrd for GribStringRef<'_> {
    fn partial_cmp(&self, other: &GribStringRef) -> Option<Ordering> {
        use self::GribStringRef::*;
        match (self, other) {
            (Ref(r1), Ref(r2)) => r1.partial_cmp(r2),
            (Char(c1), Char(c2)) => c1.partial_cmp(c2),
            (Char(c1), Ref(r2)) => {
                let mut bytes = [0u8; 4];
                let char_ref = &*c1.encode_utf8(&mut bytes);
                char_ref.partial_cmp(r2)
            }
            (Ref(r1), Char(c2)) => {
                let mut bytes = [0u8; 4];
                let char_ref = &*c2.encode_utf8(&mut bytes);
                r1.partial_cmp(&char_ref)
            }
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

    pub fn char_at(&self, ind: usize) -> Option<char> {
        match self {
            Self::Char(c) => Some(*c).filter(|_| ind == 0),
            Self::Ref(r) => r.chars().nth(ind),
        }
    }

    pub fn stringify(&self) -> String {
        match self {
            Self::Ref(r) => format!("{:?}", r),
            Self::Char(c) => {
                let mut bytes = [0u8; 4];
                format!("{:?}", c.encode_utf8(&mut bytes))
            }
        }
    }

    pub fn with_str<R, F: Fn(&str) -> R>(&self, fnc: F) -> R {
        match self {
            Self::Ref(r) => fnc(r),
            Self::Char(c) => {
                let mut bytes = [0u8; 4];
                fnc(c.encode_utf8(&mut bytes))
            }
        }
    }

    pub fn borrow(&'_ self) -> Cow<'_, str> {
        match self {
            Self::Ref(r) => (*r).into(),
            Self::Char(c) => c.to_string().into(),
        }
    }
}
