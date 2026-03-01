use serde::{Deserialize, Serialize};
use std::fmt::Debug;

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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Located<T> {
    pub data: T,
    pub start: Location,
    pub end: Location,
}

impl<T> Located<T> {
    pub fn new(data: T, start: Location, end: Location) -> Self {
        Self { data, start, end }
    }

    pub fn get_span(&self) -> LocationSpan {
        LocationSpan {
            start: self.start,
            end: self.end,
        }
    }
}

impl<'de, T: Clone + Debug + PartialEq + Deserialize<'de> + Serialize> Located<T> {
    pub fn with_loc(data: T, start: Location, end: Location) -> Located<T> {
        Located { start, end, data }
    }
}

impl<T: Copy> Copy for Located<T> {}
