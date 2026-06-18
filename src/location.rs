use derive_more::{From, Into};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, ops::Deref};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub struct ModuleInd(usize);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Location {
    row: usize,
    column: usize,
}

impl Location {
    pub fn new() -> Self {
        Self { row: 1, column: 0 }
    }

    pub fn feed(&mut self, c: char) {
        if c == '\n' {
            self.row += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }
    }

    pub fn get_row(&self) -> usize {
        self.row
    }

    pub fn get_col(&self) -> usize {
        self.column
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct LocationSpan {
    pub start: Location,
    pub end: Location,
    pub module: Option<ModuleInd>,
}

impl LocationSpan {
    pub fn new(start: Location, end: Location, m: impl Into<Option<ModuleInd>>) -> Self {
        Self {
            start,
            end,
            module: m.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Located<T> {
    pub data: T,
    pub loc: LocationSpan,
}

impl<T> Located<T> {
    pub fn new(data: T, loc: LocationSpan) -> Self {
        Self { data, loc }
    }

    pub fn get_start(&self) -> Location {
        self.loc.start
    }

    pub fn get_end(&self) -> Location {
        self.loc.end
    }
}

impl<T> Deref for Located<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: PartialEq> PartialEq for Located<T> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data && self.loc == other.loc
    }
}

impl<T: Clone> Clone for Located<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            loc: self.loc,
        }
    }
}

impl<T: Copy> Copy for Located<T> {}
