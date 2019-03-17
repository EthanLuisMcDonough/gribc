use super::node::{
    Assignable, ConditionBodyPair, Declaration, Expression, LambdaBody, LocatedOr, Node,
    ObjectValue,
};
use location::Located;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq)]
enum DefType {
    Mutable,
    Constant,
}

#[derive(Clone, Debug, Copy)]
pub enum WalkErrorType {
    ImmutableModification,
    IdentifierNotFound,
    InvalidRedefinition,
}

#[derive(Clone, Debug)]
pub struct WalkError {
    identifier: Located<String>,
    kind: WalkErrorType,
}

fn walk_ast<'a>(
    nodes: impl Iterator<Item = &'a Node> + Clone,
    mut scope: HashMap<&'a str, DefType>,
    c_scope: impl Into<Option<HashSet<&'a str>>>,
) -> Result<(), WalkError> {
    fn walk_decl<'a>(
        decl: &'a Declaration,
        scope: &mut HashMap<&'a str, DefType>,
        current_scope: &mut HashSet<&'a str>,
    ) -> Result<(), WalkError> {
        for d in &decl.declarations {
            if !current_scope.insert(&d.identifier.data) {
                return Err(WalkError {
                    identifier: d.identifier.clone(),
                    kind: WalkErrorType::InvalidRedefinition,
                });
            }
            walk_expression(&d.value, &scope)?;
            scope.insert(
                &d.identifier.data,
                if decl.mutable {
                    DefType::Mutable
                } else {
                    DefType::Constant
                },
            );
        }
        Ok(())
    }

    let mut current_scope = c_scope.into().unwrap_or_default();
    for node in nodes.clone() {
        if let Node::Procedure { identifier, .. } = node {
            if !current_scope.insert(&*identifier.data) {
                return Err(WalkError {
                    identifier: identifier.clone(),
                    kind: WalkErrorType::InvalidRedefinition,
                });
            }
            scope.insert(&identifier.data, DefType::Constant);
        }
    }

    let proc_scope = scope.clone();

    for node in nodes {
        match node {
            Node::Procedure {
                body, param_list, ..
            } => walk_ast(
                body.iter(),
                {
                    let mut scope = proc_scope.clone();
                    for param in param_list.all_params() {
                        scope.insert(param, DefType::Mutable);
                    }
                    scope
                },
                param_list.all_params().collect::<HashSet<_>>(),
            )?,
            Node::Expression(expression) | Node::Return(expression) => {
                walk_expression(expression, &scope)?
            }
            Node::Block(nodes) => walk_ast(nodes.iter(), scope.clone(), None)?,
            Node::While(ConditionBodyPair { condition, block }) => {
                walk_expression(condition, &scope)?;
                walk_ast(block.iter(), scope.clone(), None)?;
            }
            Node::Declaration(declaration) => {
                walk_decl(&declaration, &mut scope, &mut current_scope)?
            }
            Node::LogicChain {
                if_block,
                elseifs,
                else_block,
            } => {
                for ConditionBodyPair { condition, block } in
                    std::iter::once(if_block).chain(elseifs)
                {
                    walk_expression(condition, &scope)?;
                    walk_ast(block.iter(), scope.clone(), None)?;
                }
                if let Some(block) = else_block {
                    walk_ast(block.iter(), scope.clone(), None)?;
                }
            }
            Node::For {
                declaration,
                condition,
                increment,
                body,
            } => {
                let mut new_scope = scope.clone();
                let mut new_local = HashSet::new();
                if let Some(decl) = declaration {
                    walk_decl(decl, &mut new_scope, &mut new_local)?;
                }
                for expr in condition.iter().chain(increment) {
                    walk_expression(expr, &new_scope)?;
                }
                walk_ast(body.iter(), new_scope, new_local)?;
            }
            Node::Break | Node::Continue => {}
        }
    }

    Ok(())
}

fn walk_expression<'a>(
    expression: &Expression,
    scope: &HashMap<&'a str, DefType>,
) -> Result<(), WalkError> {
    fn walk_lambda_block<'a>(
        block: &LambdaBody,
        scope: &HashMap<&'a str, DefType>,
        c_scope: impl Into<Option<HashSet<&'a str>>>,
    ) -> Result<(), WalkError> {
        match block {
            LambdaBody::Block(block) => walk_ast(block.iter(), scope.clone(), c_scope.into()),
            LambdaBody::ImplicitReturn(expr) => walk_expression(expr, scope),
        }
    }

    match expression {
        Expression::Unary { expr, .. } => walk_expression(expr, scope)?,
        Expression::Binary { left, right, .. }
        | Expression::IndexAccess {
            item: left,
            index: right,
        } => {
            walk_expression(left, scope).and_then(|()| walk_expression(right, scope))?;
        }
        Expression::ArrayCreation(exprs) => {
            for e in exprs {
                walk_expression(e, scope)?;
            }
        }
        Expression::FunctionCall { function, args } => {
            walk_expression(function, scope)?;
            for e in args {
                walk_expression(e, scope)?;
            }
        }
        Expression::PropertyAccess { item, .. } => walk_expression(item, scope)?,
        Expression::Assignment { left, right, .. } => {
            match left {
                Assignable::Identifier(i) => scope
                    .get(&*i.data)
                    .ok_or(WalkErrorType::IdentifierNotFound)
                    .and_then(|&def_type| {
                        Some(def_type)
                            .filter(|t| *t == DefType::Mutable)
                            .ok_or(WalkErrorType::ImmutableModification)
                    })
                    .map_err(|kind| WalkError {
                        kind,
                        identifier: i.clone(),
                    })
                    .map(std::mem::drop)?,
                Assignable::IndexAccess { item, index } => {
                    walk_expression(item, scope).and_then(|()| walk_expression(index, scope))?
                }
                Assignable::PropertyAccess { item, .. } => walk_expression(item, scope)?,
            }
            walk_expression(right, scope)?;
        }
        Expression::Hash(hash) | Expression::MutableHash(hash) => {
            for (_, value) in hash.iter() {
                match value {
                    ObjectValue::Expression(expr) => walk_expression(expr, scope)?,
                    ObjectValue::AutoProp(a) => {
                        match a.get.as_ref() {
                            Some(LocatedOr::Or(block)) => walk_lambda_block(block, &scope, None)?,
                            Some(LocatedOr::Located(ident))
                                if !scope.contains_key(&*ident.data) =>
                            {
                                return Err(WalkError {
                                    identifier: ident.clone(),
                                    kind: WalkErrorType::IdentifierNotFound,
                                });
                            }
                            _ => {}
                        }
                        match a.set.as_ref() {
                            Some(LocatedOr::Or(set)) => {
                                let mut scope = scope.clone();
                                let mut current_scope = HashSet::new();
                                scope.insert(&*set.param, DefType::Mutable);
                                current_scope.insert(&*set.param);
                                walk_lambda_block(&set.block, &scope, current_scope)?
                            }
                            Some(LocatedOr::Located(ident))
                                if scope
                                    .get(&*ident.data)
                                    .filter(|&&t| t == DefType::Mutable)
                                    .is_none() =>
                            {
                                return Err(WalkError {
                                    identifier: ident.clone(),
                                    kind: WalkErrorType::ImmutableModification,
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Expression::Lambda { param_list, body } => {
            let mut current_scope = HashSet::new();
            let mut scope = scope.clone();
            for param in param_list.all_params() {
                scope.insert(param, DefType::Mutable);
                current_scope.insert(param);
            }
            walk_lambda_block(&body, &scope, current_scope)?
        }
        Expression::Identifier(identifier) if !scope.contains_key(&*identifier.data) => {
            return Err(WalkError {
                identifier: identifier.clone(),
                kind: WalkErrorType::IdentifierNotFound,
            });
        }
        _ => {}
    }
    Ok(())
}

pub fn ref_check(nodes: &Vec<Node>) -> Result<(), WalkError> {
    walk_ast(nodes.iter(), HashMap::new(), None)
}
