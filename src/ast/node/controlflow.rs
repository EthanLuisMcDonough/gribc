use super::Expression;
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
}

impl FlowBreak {
    pub fn new_break() -> Self {
        Self {
            kind: BreakType::Break,
            allocations: 0,
        }
    }

    pub fn new_continue() -> Self {
        Self {
            kind: BreakType::Continue,
            allocations: 0,
        }
    }

    pub fn new_return(expr: Option<Expression>) -> Self {
        Self {
            kind: BreakType::Return(expr),
            allocations: 0,
        }
    }
}
