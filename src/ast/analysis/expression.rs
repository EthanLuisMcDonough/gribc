use super::{lambda::eval_lambda, CaptureStack, Lams, Scope, WalkError, WalkErrorType, WalkResult};
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
        Expression::Unary { expr, .. } => walk_expression(expr, scope, lams, cap)?,
        Expression::Binary { left, right, .. }
        | Expression::IndexAccess {
            item: left,
            index: right,
        } => {
            walk_expression(left, scope, lams, cap)
                .and_then(|()| walk_expression(right, scope, lams, cap))?;
        }
        Expression::ArrayCreation(exprs) => {
            for e in exprs {
                walk_expression(e, scope, lams, cap)?;
            }
        }
        Expression::FunctionCall { function, args } => {
            walk_expression(function, scope, lams, cap)?;
            for e in args {
                walk_expression(e, scope, lams, cap)?;
            }
        }
        Expression::PropertyAccess { item, .. } => walk_expression(item, scope, lams, cap)?,
        Expression::Assignment { left, right, .. } => {
            match left {
                Assignable::Stack(_) => {}
                Assignable::Identifier(i) => {
                    let s = i.data;

                    if !scope.has(s, cap) {
                        return Err(WalkError {
                            kind: WalkErrorType::IdentifierNotFound(s),
                            start: i.start.clone(),
                            end: i.end.clone(),
                        });
                    } else if !scope.has_editable(s, cap) {
                        return Err(WalkError {
                            kind: WalkErrorType::ImmutableModification(s),
                            start: i.start.clone(),
                            end: i.end.clone(),
                        });
                    }

                    let stat = scope.runtime_value(s);
                    if let Some(RuntimeValue::Stack(ptr)) = stat {
                        *left = Assignable::Stack(ptr);
                    } else {
                        panic!(
                            "Static value is not valid.  This area should be unreachable: {:?}",
                            stat
                        );
                    }
                }
                Assignable::IndexAccess { item, index } => walk_expression(item, scope, lams, cap)
                    .and_then(|()| walk_expression(index, scope, lams, cap))?,
                Assignable::PropertyAccess { item, .. } => walk_expression(item, scope, lams, cap)?,
            }
            walk_expression(right, scope, lams, cap)?;
        }
        Expression::Hash(hash) | Expression::MutableHash(hash) => {
            for (_, value) in hash.iter_mut() {
                match value {
                    ObjectValue::Expression(expr) => walk_expression(expr, scope, lams, cap)?,
                    ObjectValue::AutoProp(auto) => {
                        match auto.get.clone() {
                            Some(AutoPropValue::Lambda(ind)) => {
                                let mut lambda = mem::take(&mut lams.getters[ind]);
                                eval_lambda(&mut lambda, scope, cap, lams)?;
                                lams.getters[ind] = lambda;
                            }
                            Some(AutoPropValue::String(ident)) => {
                                if !scope.prop_check(ident.data) {
                                    return Err(WalkError {
                                        start: ident.start,
                                        end: ident.end,
                                        kind: WalkErrorType::IdentifierNotFound(ident.data),
                                    });
                                }

                                let op = scope.runtime_value(ident.data);
                                auto.get = op.map(AutoPropValue::Value);
                            }
                            _ => {}
                        }
                        match auto.set.clone() {
                            Some(AutoPropValue::Lambda(ind)) => {
                                let mut lambda = mem::take(&mut lams.setters[ind]);
                                eval_lambda(&mut lambda, scope, cap, lams)?;
                                lams.setters[ind] = lambda;
                            }
                            Some(AutoPropValue::String(ident)) => {
                                if !scope.prop_check(ident.data) {
                                    return Err(WalkError {
                                        start: ident.start.clone(),
                                        end: ident.end.clone(),
                                        kind: WalkErrorType::IdentifierNotFound(ident.data),
                                    });
                                } else if !scope.prop_check_mut(ident.data) {
                                    return Err(WalkError {
                                        start: ident.start.clone(),
                                        end: ident.end.clone(),
                                        kind: WalkErrorType::ImmutableModification(ident.data),
                                    });
                                }

                                let op = scope.runtime_value(ident.data);
                                auto.set = op.map(AutoPropValue::Value);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Expression::Lambda(ind) => {
            let mut lambda = mem::take(&mut lams.lambdas[*ind]);
            eval_lambda(&mut lambda, scope, cap, lams)?;
            lams.lambdas[*ind] = lambda;
        }
        Expression::Identifier(identifier) => {
            if !scope.has(identifier.data, cap) {
                return Err(WalkError {
                    start: identifier.start.clone(),
                    end: identifier.end.clone(),
                    kind: WalkErrorType::IdentifierNotFound(identifier.data),
                });
            }

            if let Some(val) = scope.runtime_value(identifier.data) {
                *expression = Expression::Value(val);
            }
        }
        Expression::This { start, end } if scope.lam_pass.is_none() => {
            return Err(WalkError {
                kind: WalkErrorType::InvalidThis,
                start: start.clone(),
                end: end.clone(),
            });
        }
        _ => {}
    }
    Ok(())
}
