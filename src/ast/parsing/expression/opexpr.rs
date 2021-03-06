use ast::node::{Assignable, Expression};
use location::Location;
use operators::{Assignment, Binary, Unary};
use std::convert::TryFrom;

#[derive(Debug, PartialEq)]
pub enum OpExpr {
    Binary(Binary),
    Unary(Unary),
    Assign(Assignment),
    Expr(Expression, Location),
}

impl OpExpr {
    fn is_minus(&self) -> bool {
        *self == OpExpr::Binary(Binary::Minus)
    }

    fn can_be_unary(&self) -> bool {
        self.is_unary() || self.is_minus()
    }

    pub fn is_binary(&self) -> bool {
        if let OpExpr::Binary(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_unary(&self) -> bool {
        if let OpExpr::Unary(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_assign(&self) -> bool {
        if let OpExpr::Assign(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_expr(&self) -> bool {
        if let OpExpr::Expr(_, _) = self {
            true
        } else {
            false
        }
    }

    fn unary_adjust_negation(self) -> Self {
        if self.is_minus() {
            OpExpr::Unary(Unary::Negation)
        } else {
            self
        }
    }
}

impl From<Binary> for OpExpr {
    fn from(b: Binary) -> Self {
        OpExpr::Binary(b)
    }
}

impl From<Unary> for OpExpr {
    fn from(u: Unary) -> Self {
        OpExpr::Unary(u)
    }
}

impl From<(Expression, Location)> for OpExpr {
    fn from(e: (Expression, Location)) -> Self {
        OpExpr::Expr(e.0, e.1)
    }
}

impl From<Assignment> for OpExpr {
    fn from(e: Assignment) -> Self {
        OpExpr::Assign(e)
    }
}

pub struct OpExprManager(Vec<OpExpr>);

impl OpExprManager {
    pub fn new() -> Self {
        OpExprManager(vec![])
    }

    fn can_push_expr(&self) -> bool {
        self.0.last().filter(|m| m.is_expr()).is_none()
    }

    fn can_push_binary(&self) -> bool {
        !self.can_push_expr()
    }

    fn can_push_unary(&self) -> bool {
        self.can_push_expr()
    }

    pub fn push(&mut self, op_expr: impl Into<OpExpr>) -> Result<(), OpExpr> {
        let op_expr = op_expr.into();

        if op_expr.can_be_unary() && self.can_push_unary() {
            self.0.push(op_expr.unary_adjust_negation())
        } else if ((op_expr.is_binary() || op_expr.is_assign()) && self.can_push_binary())
            || (op_expr.is_expr() && self.can_push_expr())
        {
            self.0.push(op_expr);
        } else {
            return Err(op_expr);
        }

        Ok(())
    }
}

impl From<OpExprManager> for Vec<OpExpr> {
    fn from(m: OpExprManager) -> Self {
        m.0
    }
}

impl TryFrom<Expression> for Assignable {
    type Error = Expression;

    fn try_from(e: Expression) -> Result<Self, Self::Error> {
        match e {
            Expression::Identifier(s) => Ok(Assignable::Identifier(s)),
            Expression::IndexAccess { item, index } => Ok(Assignable::IndexAccess { item, index }),
            Expression::PropertyAccess { item, property } => {
                Ok(Assignable::PropertyAccess { item, property })
            }
            _ => Err(e),
        }
    }
}
