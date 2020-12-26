mod scope;

use ast::node::{
    Assignable, ConditionBodyPair, Declaration, Expression, LambdaBody, LocatedOr, Node,
    ObjectValue, Import, Procedure, ImportKind, Module
};
use location::Located;
use self::scope::Scope;

type WalkResult = Result<(), WalkError>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Copy)]
pub enum WalkErrorType {
    ImmutableModification,
    IdentifierNotFound,
    InvalidRedefinition,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WalkError {
    identifier: Located<String>,
    kind: WalkErrorType,
}

// Inserts import items into scope
fn module_register<'a>(import: &'a Import, scope: &mut Scope<'a>) {
    match &import.kind {
        ImportKind::All => {
            for key in import.module.get_functions() {
                scope.insert_fn(key);
            }
        },
        ImportKind::ModuleObject(Located { data, .. }) => {
            scope.insert_fn(data.as_str());
        },
        ImportKind::List(l) => {
            for key in l.keys() {
                scope.insert_fn(key);
            }
        },
    }
}

fn walk_import(import: &Import) -> WalkResult {
    let module = &import.module;
    walk_module(module)?;

    if let ImportKind::List(l) = &import.kind {
        for (ident, (start, end)) in l {
            if !module.has_function(ident) {
                return Err(WalkError {
                    identifier: Located { 
                        data: ident.clone(), 
                        start: start.clone(), 
                        end: end.clone() 
                    },
                    kind: WalkErrorType::IdentifierNotFound
                });
            }
        }
    }

    Ok(())
}

fn walk_module(package: &Module) -> WalkResult {
    let module = match package {
        Module::Native(_) => return Ok(()),
        Module::Custom(m) => m,
    };

    let mut scope = Scope::new();

    for import in &module.imports {
        walk_import(import)?;
        module_register(import, &mut scope);
    }

    for Procedure { identifier, .. } in &module.functions {
        if !scope.insert_fn(identifier.data.as_str()) {
            return Err(WalkError {
                identifier: identifier.clone(),
                kind: WalkErrorType::InvalidRedefinition,
            });
        }
    }

    for p in &module.functions {
        walk_procedure(p, &scope)?;
    }
    
    Ok(())
}

fn walk_decl<'a>(
    decl: &'a Declaration,
    scope: &mut Scope<'a>,
) -> Result<(), WalkError> {
    for d in &decl.declarations {
        walk_expression(&d.value, &scope)?;
        if !scope.insert_var(&d.identifier.data, decl.mutable) {
            return Err(WalkError {
                identifier: d.identifier.clone(),
                kind: WalkErrorType::InvalidRedefinition,
            });
        }
    }
    Ok(())
}

fn walk_ast<'a>(
    nodes: impl Iterator<Item = &'a Node> + Clone,
    mut scope: Scope<'a>,
) -> Result<(), WalkError> {
    
    for node in nodes.clone() {
        if let Node::Procedure(Procedure { identifier, .. }) = node {
            if !scope.insert_fn(&identifier.data) {
                return Err(WalkError {
                    identifier: identifier.clone(),
                    kind: WalkErrorType::InvalidRedefinition,
                });
            }
        } else if let Node::Import(import) = node {
            walk_import(import)?;
            module_register(import, &mut scope);
        }
    }

    let proc_scope = scope.proc_scope();

    for node in nodes {
        match node {
            Node::Procedure(p) => walk_procedure(p, &proc_scope)?,
            Node::Expression(expression) | Node::Return(expression) => {
                walk_expression(expression, &scope)?
            }
            Node::Block(nodes) => walk_ast(nodes.iter(), scope.sub())?,
            Node::While(ConditionBodyPair { condition, block }) => {
                walk_expression(condition, &scope)?;
                walk_ast(block.iter(), scope.sub())?;
            }
            Node::Declaration(declaration) => {
                walk_decl(&declaration, &mut scope)?
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
                    walk_ast(block.iter(), scope.sub())?;
                }
                if let Some(block) = else_block {
                    walk_ast(block.iter(), scope.sub())?;
                }
            }
            Node::For {
                declaration,
                condition,
                increment,
                body,
            } => {
                let mut new_scope = scope.sub();
                if let Some(decl) = declaration {
                    walk_decl(decl, &mut new_scope)?;
                }
                for expr in condition.iter().chain(increment) {
                    walk_expression(expr, &new_scope)?;
                }
                walk_ast(body.iter(), new_scope)?;
            }
            Node::Import(_) => {} // Imports are managed with function definitions
            Node::Break | Node::Continue => {}
        }
    }

    Ok(())
}

fn walk_lambda_block<'a>(
    block: &LambdaBody,
    scope: Scope<'a>
) -> Result<(), WalkError> {
    match block {
        LambdaBody::Block(block) => walk_ast(block.iter(), scope),
        LambdaBody::ImplicitReturn(expr) => walk_expression(expr, &scope),
    }
}


fn walk_procedure<'a>(
    procedure: &Procedure,
    scope: &Scope<'a>,
) -> Result<(), WalkError> {
    walk_ast(
        procedure.body.iter(),
        {
            let mut scope = scope.clone();
            for param in procedure.param_list.all_params() {
                scope.insert_mut(param);
            }
            scope
        }
    )
}

fn walk_expression<'a>(
    expression: &Expression,
    scope: &Scope<'a>,
) -> Result<(), WalkError> {
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
                Assignable::Identifier(i) => {
                    let s = i.data.as_ref();

                    if !scope.has(s) {
                        return Err(WalkError {
                            kind: WalkErrorType::IdentifierNotFound,
                            identifier: i.clone(),
                        });
                    } else if !scope.has_editable(s) {
                        return Err(WalkError {
                            kind: WalkErrorType::ImmutableModification,
                            identifier: i.clone(),
                        });
                    }
                }
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
                            Some(LocatedOr::Or(block)) => walk_lambda_block(block, scope.sub())?,
                            Some(LocatedOr::Located(ident)) 
                                if !scope.has(&*ident.data) => return Err(WalkError {
                                    identifier: ident.clone(),
                                    kind: WalkErrorType::IdentifierNotFound,
                                }),
                            _ => {} 
                        }
                        match a.set.as_ref() {
                            Some(LocatedOr::Or(set)) => {
                                let mut scope = scope.clone();
                                scope.insert_mut(set.param.as_str());
                                walk_lambda_block(&set.block, scope)?
                            }
                            Some(LocatedOr::Located(ident)) if !scope.has(ident.data.as_str()) => {
                                return Err(WalkError {
                                    identifier: ident.clone(),
                                    kind: WalkErrorType::IdentifierNotFound,
                                });
                            }
                            Some(LocatedOr::Located(ident))
                                if !scope.has_editable(ident.data.as_str()) =>
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
            let mut scope = scope.clone();
            for param in param_list.all_params() {
                scope.insert_mut(param);
            }
            walk_lambda_block(&body, scope)?
        }
        Expression::Identifier(identifier) if !scope.has(&*identifier.data) => {
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
    walk_ast(nodes.iter(), Scope::new())
}
