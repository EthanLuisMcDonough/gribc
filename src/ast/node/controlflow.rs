use super::Expression;
use location::Location;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum BreakType {
    Break,
    Continue,
    Return(Option<Expression>),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct FlowBreak {
    pub allocations: usize,
    pub kind: BreakType,
    pub start: Location,
    pub end: Location,
}

impl FlowBreak {
    pub fn new_break(start: Location, end: Location) -> Self {
        Self {
            kind: BreakType::Break,
            allocations: 0,
            start,
            end,
        }
    }

    pub fn new_continue(start: Location, end: Location) -> Self {
        Self {
            kind: BreakType::Continue,
            allocations: 0,
            start,
            end,
        }
    }

    pub fn new_return(expr: Option<Expression>, start: Location, end: Location) -> Self {
        Self {
            kind: BreakType::Return(expr),
            allocations: 0,
            start,
            end,
        }
    }
}
