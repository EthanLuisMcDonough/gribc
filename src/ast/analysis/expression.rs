use super::{
    lambda::eval_lambda, CaptureStack, IndexAccess, Lams, PropertyAccess, Scope, WalkError,
    WalkErrorType, WalkResult,
};
use ast::node::*;
use std::mem;

/// Expression walking
pub fn walk_expression(
    expression: &mut Expression,
    scope: &mut Scope,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> WalkResult {
    match expression {
        Expression::Unary(UnaryExpression { expr, .. }) => walk_expression(expr, scope, lams, cap)?,
        Expression::Binary(BinaryExpression { left, right, .. })
        | Expression::IndexAccess(IndexAccess {
            item: left,
            index: right,
        }) => {
            walk_expression(left, scope, lams, cap)
                .and_then(|()| walk_expression(right, scope, lams, cap))?;
        }
        Expression::ArrayCreation(exprs) => {
            for e in exprs {
                walk_expression(e, scope, lams, cap)?;
            }
        }
        Expression::FunctionCall(CallExpression { function, args }) => {
            walk_expression(function, scope, lams, cap)?;
            for e in args {
                walk_expression(e, scope, lams, cap)?;
            }
        }
        Expression::PropertyAccess(PropertyAccess { item, .. }) => {
            walk_expression(item, scope, lams, cap)?
        }
        Expression::Assignment(AssignmentExpression { left, right, .. }) => {
            match left {
                Assignable::Offset(_) => {}
                Assignable::Identifier(i) => {
                    let s = i.data;

                    if scope.in_first_pass() {
                        if !scope.has(s, cap) {
                            return Err(WalkError {
                                data: WalkErrorType::IdentifierNotFound(s),
                                loc: i.loc,
                            });
                        } else if !scope.has_editable(s, cap) {
                            return Err(WalkError {
                                data: WalkErrorType::ImmutableModification(s),
                                loc: i.loc,
                            });
                        }
                    }

                    if scope.in_second_pass() {
                        let stat = scope.runtime_value(s);
                        if let Some(RuntimeValue::StackOffset(offset)) = stat {
                            *left = Assignable::Offset(offset);
                        } else {
                            panic!(
                                "Static value is not valid.  This area should be unreachable: {:?}",
                                stat
                            );
                        }
                    }
                }
                Assignable::IndexAccess(IndexAccess { item, index }) => {
                    walk_expression(item, scope, lams, cap)
                        .and_then(|()| walk_expression(index, scope, lams, cap))?
                }
                Assignable::PropertyAccess(PropertyAccess { item, .. }) => {
                    walk_expression(item, scope, lams, cap)?
                }
            }
            walk_expression(right, scope, lams, cap)?;
        }
        Expression::Hash(hash) | Expression::MutableHash(hash) => {
            for (_, value) in hash.iter_mut() {
                match value {
                    ObjectValue::Expression(expr) => walk_expression(expr, scope, lams, cap)?,
                    ObjectValue::AutoProp(auto) => {
                        match auto.get.clone() {
                            Some(GetAutoPropValue::Lambda(ind)) if scope.in_first_pass() => {
                                let mut lambda = mem::take(&mut lams.getters[ind]);
                                eval_lambda(&mut lambda, scope, cap, lams)?;
                                lams.getters[ind] = lambda;
                            }
                            Some(GetAutoPropValue::String(ident)) => {
                                if scope.in_first_pass() && !scope.prop_check(ident.data) {
                                    return Err(WalkError {
                                        data: WalkErrorType::IdentifierNotFound(ident.data),
                                        loc: ident.loc,
                                    });
                                }
                                if scope.in_second_pass() {
                                    let op = scope.runtime_value(ident.data);
                                    auto.get = op.map(GetAutoPropValue::Value);
                                }
                            }
                            _ => {}
                        }
                        match auto.set.clone() {
                            Some(SetAutoPropValue::Lambda(ind)) if scope.in_first_pass() => {
                                let mut lambda = mem::take(&mut lams.setters[ind]);
                                eval_lambda(&mut lambda, scope, cap, lams)?;
                                lams.setters[ind] = lambda;
                            }
                            Some(SetAutoPropValue::String(ident)) => {
                                if scope.in_first_pass() {
                                    if !scope.prop_check(ident.data) {
                                        return Err(WalkError {
                                            data: WalkErrorType::IdentifierNotFound(ident.data),
                                            loc: ident.loc,
                                        });
                                    } else if !scope.prop_check_mut(ident.data) {
                                        return Err(WalkError {
                                            data: WalkErrorType::ImmutableModification(ident.data),
                                            loc: ident.loc,
                                        });
                                    }
                                }
                                if scope.in_second_pass() {
                                    match scope.runtime_value(ident.data) {
                                        Some(RuntimeValue::StackOffset(offset)) => {
                                            auto.set = SetAutoPropValue::StackOffset(offset).into();
                                        }
                                        Some(RuntimeValue::Static(_)) => {
                                            unreachable!("Static value found in setter");
                                        }
                                        None => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        // Nested lambdas are only visited once
        Expression::Lambda(ind) if scope.in_first_pass() => {
            let mut lambda = mem::take(&mut lams.lambdas[*ind]);
            eval_lambda(&mut lambda, scope, cap, lams)?;
            lams.lambdas[*ind] = lambda;
        }
        Expression::Identifier(identifier) => {
            if scope.in_first_pass() && !scope.has(identifier.data, cap) {
                return Err(WalkError {
                    data: WalkErrorType::IdentifierNotFound(identifier.data),
                    loc: identifier.loc,
                });
            }
            if scope.in_second_pass() {
                if let Some(val) = scope.runtime_value(identifier.data) {
                    *expression = Expression::Value(val);
                }
            }
        }
        Expression::This(span) if scope.lam_pass.is_none() => {
            return Err(WalkError {
                data: WalkErrorType::InvalidThis,
                loc: *span,
            });
        }
        _ => {}
    }
    Ok(())
}
