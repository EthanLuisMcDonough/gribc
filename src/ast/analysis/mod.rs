mod scope;

use self::scope::*;
use ast::node::*;
use location::Located;
use std::collections::HashSet;
use std::mem;

type WalkResult = Result<(), WalkError>;
type Lambdas = Vec<Lambda>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Copy)]
pub enum WalkErrorType {
    ImmutableModification,
    IdentifierNotFound,
    InvalidRedefinition,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WalkError {
    identifier: Located<usize>,
    kind: WalkErrorType,
}

pub struct Lams<'a> {
    lambdas: &'a mut Lambdas,
    getters: &'a mut Vec<GetProp>,
    setters: &'a mut Vec<SetProp>,
}

// Inserts import items into scope
fn module_register<'a>(
    import: &'a Import,
    functions: &HashSet<usize>,
    scope: &mut Scope,
) -> WalkResult {
    match &import.kind {
        ImportKind::All => {
            for key in functions {
                scope.insert_import(*key);
            }
        }
        ImportKind::ModuleObject(Located { data, .. }) => {
            scope.insert_import(*data);
        }
        ImportKind::List(l) => {
            for (key, (start, end)) in l {
                if !functions.contains(key) {
                    return Err(WalkError {
                        identifier: Located {
                            data: key.clone(),
                            start: start.clone(),
                            end: end.clone(),
                        },
                        kind: WalkErrorType::IdentifierNotFound,
                    });
                }

                scope.insert_import(*key);
            }
        }
    }

    Ok(())
}

fn walk_import<'a>(import: &'a Import, modules: &'a ModuleStore, scope: &mut Scope) -> WalkResult {
    let module = &import.module;

    let functions = match module {
        Module::Native { indices, .. } => indices.iter().map(|e| *e).collect(),
        Module::Custom(p) => modules[*p].get_functions(),
    };

    module_register(import, &functions, scope)
}

fn walk_module(
    module: &CustomModule,
    modules: &ModuleStore,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> WalkResult {
    let mut scope = Scope::new();

    for import in &module.imports {
        walk_import(import, modules, &mut scope)?;
    }

    for Procedure { identifier, .. } in &module.functions {
        if !scope.insert_fn(identifier.data) {
            return Err(WalkError {
                identifier: identifier.clone(),
                kind: WalkErrorType::InvalidRedefinition,
            });
        }
    }

    for p in &module.functions {
        walk_procedure(p, &scope, lams, cap)?;
    }

    Ok(())
}

fn walk_decl<'a>(
    decl: &'a Declaration,
    scope: &mut Scope,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> Result<(), WalkError> {
    for d in &decl.declarations {
        walk_expression(&d.value, &scope, lams, cap)?;
        if !scope.insert_var(d.identifier.data, decl.mutable) {
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
    mut scope: Scope,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> Result<(), WalkError> {
    for node in nodes {
        match node {
            Node::Expression(expression) | Node::Return(expression) => {
                walk_expression(expression, &scope, lams, cap)?
            }
            Node::Block(nodes) => walk_ast(nodes.iter(), scope.sub(), lams, cap)?,
            Node::While(ConditionBodyPair { condition, block }) => {
                walk_expression(condition, &scope, lams, cap)?;
                walk_ast(block.iter(), scope.sub(), lams, cap)?;
            }
            Node::Declaration(declaration) => walk_decl(&declaration, &mut scope, lams, cap)?,
            Node::LogicChain {
                if_block,
                elseifs,
                else_block,
            } => {
                for ConditionBodyPair { condition, block } in
                    std::iter::once(if_block).chain(elseifs)
                {
                    walk_expression(condition, &scope, lams, cap)?;
                    walk_ast(block.iter(), scope.sub(), lams, cap)?;
                }
                if let Some(block) = else_block {
                    walk_ast(block.iter(), scope.sub(), lams, cap)?;
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
                    walk_decl(decl, &mut new_scope, lams, cap)?;
                }
                for expr in condition.iter().chain(increment) {
                    walk_expression(expr, &new_scope, lams, cap)?;
                }
                walk_ast(body.iter(), new_scope, lams, cap)?;
            }
            Node::Break | Node::Continue => {}
        }
    }

    Ok(())
}

fn walk_lambda_block<'a>(
    block: &LambdaBody,
    scope: Scope,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> Result<(), WalkError> {
    match block {
        LambdaBody::Block(block) => walk_ast(block.iter(), scope, lams, cap),
        LambdaBody::ImplicitReturn(expr) => walk_expression(expr, &scope, lams, cap),
    }
}

fn walk_procedure<'a>(
    procedure: &Procedure,
    scope: &Scope,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> Result<(), WalkError> {
    walk_ast(
        procedure.body.iter(),
        {
            let mut scope = scope.clone();
            for param in procedure.param_list.all_params() {
                scope.insert_mut(*param);
            }
            scope
        },
        lams,
        cap,
    )
}

fn walk_expression<'a>(
    expression: &Expression,
    scope: &Scope,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> Result<(), WalkError> {
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
                Assignable::Identifier(i) => {
                    let s = i.data;

                    if !scope.has(s, cap) {
                        return Err(WalkError {
                            kind: WalkErrorType::IdentifierNotFound,
                            identifier: i.clone(),
                        });
                    } else if !scope.has_editable(s, cap) {
                        return Err(WalkError {
                            kind: WalkErrorType::ImmutableModification,
                            identifier: i.clone(),
                        });
                    }
                }
                Assignable::IndexAccess { item, index } => walk_expression(item, scope, lams, cap)
                    .and_then(|()| walk_expression(index, scope, lams, cap))?,
                Assignable::PropertyAccess { item, .. } => walk_expression(item, scope, lams, cap)?,
            }
            walk_expression(right, scope, lams, cap)?;
        }
        Expression::Hash(hash) | Expression::MutableHash(hash) => {
            for (_, value) in hash.iter() {
                match value {
                    ObjectValue::Expression(expr) => walk_expression(expr, scope, lams, cap)?,
                    ObjectValue::AutoProp(auto) => {
                        match auto.get.as_ref() {
                            Some(AutoPropValue::Lambda(ind)) => {
                                let scope = scope.sub();
                                cap.add(scope.level);

                                let mut get = mem::take(&mut lams.getters[*ind]);
                                walk_lambda_block(&get.block, scope, lams, cap)?;
                                get.capture = cap.pop();

                                lams.getters[*ind] = get;
                            }
                            Some(AutoPropValue::String(ident)) if !scope.has(ident.data, cap) => {
                                return Err(WalkError {
                                    identifier: ident.clone(),
                                    kind: WalkErrorType::IdentifierNotFound,
                                })
                            }
                            _ => {}
                        }
                        match auto.set.as_ref() {
                            Some(AutoPropValue::Lambda(ind)) => {
                                let mut scope = scope.sub();
                                cap.add(scope.level);

                                let mut set = mem::take(&mut lams.setters[*ind]);
                                scope.insert_mut(set.param);

                                walk_lambda_block(&set.block, scope, lams, cap)?;
                                set.capture = cap.pop();

                                lams.setters[*ind] = set;
                            }
                            Some(AutoPropValue::String(ident)) if !scope.has(ident.data, cap) => {
                                return Err(WalkError {
                                    identifier: ident.clone(),
                                    kind: WalkErrorType::IdentifierNotFound,
                                });
                            }
                            Some(AutoPropValue::String(ident))
                                if !scope.has_editable(ident.data, cap) =>
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
        Expression::Lambda(ind) => {
            let mut lambda = mem::take(&mut lams.lambdas[*ind]);
            let mut scope = scope.sub();

            cap.add(scope.level);

            for param in lambda.param_list.all_params() {
                scope.insert_mut(*param);
            }
            walk_lambda_block(&lambda.body, scope, lams, cap)?;

            lambda.captured = cap.pop();
            lams.lambdas[*ind] = lambda;
        }
        Expression::Identifier(identifier) if !scope.has(identifier.data, cap) => {
            return Err(WalkError {
                identifier: identifier.clone(),
                kind: WalkErrorType::IdentifierNotFound,
            });
        }
        _ => {}
    }
    Ok(())
}

pub fn ref_check(program: &mut Program) -> Result<(), WalkError> {
    let body = &program.body;
    let modules = &program.modules;

    let mut scope = Scope::new();
    let mut stack = CaptureStack::new();

    let mut lambdas = Lams {
        lambdas: &mut program.lambdas,
        getters: &mut program.getters,
        setters: &mut program.setters,
    };

    for module in modules {
        walk_module(module, modules, &mut lambdas, &mut stack)?;
    }

    for import in &program.imports {
        walk_import(import, &program.modules, &mut scope)?;
    }

    for Procedure { identifier, .. } in &program.functions {
        if !scope.insert_fn(identifier.data) {
            return Err(WalkError {
                identifier: identifier.clone(),
                kind: WalkErrorType::InvalidRedefinition,
            });
        }
    }

    for function in &program.functions {
        walk_procedure(function, &scope, &mut lambdas, &mut stack)?;
    }

    walk_ast(body.iter(), scope, &mut lambdas, &mut stack)
}
