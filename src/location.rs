use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Located<T> {
    pub data: T,
    pub start: Location,
    pub end: Location,
}

impl<T: Clone + Debug + PartialEq + DeserializeOwned + Serialize> Located<T> {
    pub fn with_loc(data: T, start: Location, end: Location) -> Located<T> {
        Located { start, end, data }
    }
}
