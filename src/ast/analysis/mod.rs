mod scope;

use self::scope::*;
use ast::node::*;
use location::Located;
use std::collections::HashSet;
use std::mem;

pub type WalkResult = Result<(), WalkError>;
type Lambdas = Vec<Lambda>;
type Strings<'a> = &'a Vec<String>;

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

fn walk_import(
    import: &Import,
    modules: &ModuleStore,
    strings: Strings,
    scope: &mut Scope,
) -> WalkResult {
    match &import.kind {
        ImportKind::All => {
            match &import.module {
                Module::Custom(mod_ind) => {
                    for (fn_ind, proc) in modules[*mod_ind].pub_functions().enumerate() {
                        scope.import_function(proc.identifier.data, *mod_ind, fn_ind);
                    }
                }
                Module::Native(_pkg) => {
                    panic!("Branch should be unreachable.  Native all imports are rewritten");
                }
            };
        }
        ImportKind::ModuleObject(Located { data, .. }) => {
            scope.import_module(*data, import.module.clone());
        }
        ImportKind::List(list) => {
            let mut inserted = HashSet::with_capacity(list.len());

            for located in list {
                let name = located.data;

                if inserted.contains(&name) {
                    return Err(WalkError {
                        identifier: located.clone(),
                        kind: WalkErrorType::InvalidRedefinition,
                    });
                }
                inserted.insert(name);

                let contains = match &import.module {
                    Module::Custom(ind) => modules[*ind].get_function(name).is_some(),
                    Module::Native(pkg) => pkg.fn_from_str(&*strings[name]).is_some(),
                };

                if !contains {
                    return Err(WalkError {
                        identifier: located.clone(),
                        kind: WalkErrorType::IdentifierNotFound,
                    });
                }

                match &import.module {
                    Module::Custom(mod_ind) => {
                        if let Some(fnc_ind) = modules[*mod_ind].get_function(name) {
                            scope.import_function(name, *mod_ind, fnc_ind);
                        }
                    }
                    Module::Native(pkg) => {
                        if let Some(fnc) = pkg.fn_from_str(&*strings[name]) {
                            scope.native_function(name, fnc.clone());
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn walk_module(
    module: &mut CustomModule,
    module_ind: usize,
    modules: &ModuleStore,
    lams: &mut Lams,
    cap: &mut CaptureStack,
    strings: Strings,
) -> WalkResult {
    let mut scope = Scope::new();

    for import in &module.imports {
        walk_import(import, modules, strings, &mut scope)?;
    }

    for (ind, Procedure { identifier, .. }) in module.functions.iter().enumerate() {
        if !scope.insert_fn(identifier.data, ind, Some(module_ind)) {
            return Err(WalkError {
                identifier: identifier.clone(),
                kind: WalkErrorType::InvalidRedefinition,
            });
        }
    }

    for p in &mut module.functions {
        walk_procedure(p, &mut scope, lams, cap)?;
    }

    Ok(())
}

fn walk_decl(
    decl: &mut Declaration,
    scope: &mut Scope,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> Result<(), WalkError> {
    for d in decl.declarations.iter_mut() {
        walk_expression(&mut d.value, scope, lams, cap)?;
        if !scope.insert_var(d.identifier.data, decl.mutable) {
            return Err(WalkError {
                identifier: d.identifier.clone(),
                kind: WalkErrorType::InvalidRedefinition,
            });
        }
    }
    Ok(())
}

fn walk_ast(
    nodes: &mut Block,
    scope: &mut Scope,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> Result<(), WalkError> {
    for node in nodes.iter_mut() {
        match node {
            Node::Expression(expression) | Node::Return(expression) => {
                walk_expression(expression, scope, lams, cap)?
            }
            Node::Block(nodes) => {
                scope.sub_block(|sub, nodes| walk_ast(nodes, sub, lams, cap), nodes)?;
            }
            Node::While(ConditionBodyPair { condition, block }) => {
                walk_expression(condition, scope, lams, cap)?;
                scope.sub_block(|sub, block| walk_ast(block, sub, lams, cap), block)?;
            }
            Node::Declaration(declaration) => walk_decl(declaration, scope, lams, cap)?,
            Node::LogicChain {
                if_block,
                elseifs,
                else_block,
            } => {
                for ConditionBodyPair { condition, block } in
                    std::iter::once(if_block).chain(elseifs)
                {
                    walk_expression(condition, scope, lams, cap)?;
                    scope.sub_block(|scope, block| walk_ast(block, scope, lams, cap), block)?;
                }
                if let Some(block) = else_block {
                    scope.sub_block(|scope, block| walk_ast(block, scope, lams, cap), block)?;
                }
            }
            Node::For {
                declaration,
                condition,
                increment,
                body,
            } => {
                scope.sub_block(
                    |new_scope, body| {
                        if let Some(decl) = declaration {
                            walk_decl(decl, new_scope, lams, cap)?;
                        }
                        if let Some(expr) = condition {
                            walk_expression(expr, new_scope, lams, cap)?;
                        }
                        if let Some(expr) = increment {
                            walk_expression(expr, new_scope, lams, cap)?;
                        }
                        walk_ast(body, new_scope, lams, cap)
                    },
                    body,
                )?;
            }
            Node::Break | Node::Continue => {}
        }
    }

    Ok(())
}

fn walk_lambda_block(
    block: &mut LambdaBody,
    scope: &mut Scope,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> Result<(), WalkError> {
    match block {
        LambdaBody::Block(block) => {
            let res = walk_ast(block, scope, lams, cap);
            scope.check_decls(block);
            res
        }
        LambdaBody::ImplicitReturn(expr) => walk_expression(expr, scope, lams, cap),
    }
}

fn walk_procedure(
    procedure: &mut Procedure,
    scope: &mut Scope,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> Result<(), WalkError> {
    scope.sub_fnc(
        |scope, _, body| walk_ast(body, scope, lams, cap),
        &mut procedure.param_list,
        &mut procedure.body,
    )
}

fn walk_expression(
    expression: &mut Expression,
    scope: &mut Scope,
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
            for (_, value) in hash.iter_mut() {
                match value {
                    ObjectValue::Expression(expr) => walk_expression(expr, scope, lams, cap)?,
                    ObjectValue::AutoProp(auto) => {
                        scope.sub(|scope| match auto.get.as_ref() {
                            Some(AutoPropValue::Lambda(ind)) => {
                                cap.add(scope.level);

                                let mut get = mem::take(&mut lams.getters[*ind]);
                                walk_lambda_block(&mut get.block, scope, lams, cap)?;
                                get.capture = cap.pop();

                                lams.getters[*ind] = get;
                                Ok(())
                            }
                            Some(AutoPropValue::String(ident)) if !scope.prop_check(ident.data) => {
                                Err(WalkError {
                                    identifier: ident.clone(),
                                    kind: WalkErrorType::IdentifierNotFound,
                                })
                            }
                            Some(AutoPropValue::String(_)) | None => Ok(()),
                        })?;
                        scope.sub(|scope| match auto.set.as_ref() {
                            Some(AutoPropValue::Lambda(ind)) => {
                                cap.add(scope.level);

                                let mut set = mem::take(&mut lams.setters[*ind]);
                                scope.insert_mut(set.param);

                                walk_lambda_block(&mut set.block, scope, lams, cap)?;

                                set.capture = cap.pop();
                                set.param_captured = scope.is_captured(set.param);

                                lams.setters[*ind] = set;
                                Ok(())
                            }
                            Some(AutoPropValue::String(ident)) if !scope.prop_check(ident.data) => {
                                Err(WalkError {
                                    identifier: ident.clone(),
                                    kind: WalkErrorType::IdentifierNotFound,
                                })
                            }
                            Some(AutoPropValue::String(ident))
                                if !scope.prop_check_mut(ident.data) =>
                            {
                                Err(WalkError {
                                    identifier: ident.clone(),
                                    kind: WalkErrorType::ImmutableModification,
                                })
                            }
                            _ => Ok(()),
                        })?;
                    }
                }
            }
        }
        Expression::Lambda(ind) => {
            let mut lambda = mem::take(&mut lams.lambdas[*ind]);
            let body = &mut lambda.body;
            scope.sub_params(
                |scope, _params| {
                    cap.add(scope.level);
                    walk_lambda_block(body, scope, lams, cap)
                },
                &mut lambda.param_list,
            )?;
            lambda.captured = cap.pop();
            lams.lambdas[*ind] = lambda;
        }
        Expression::Identifier(identifier) => {
            if !scope.has(identifier.data, cap) {
                return Err(WalkError {
                    identifier: identifier.clone(),
                    kind: WalkErrorType::IdentifierNotFound,
                });
            } else {
                if let Some(val) = scope.try_static(identifier.data) {
                    *expression = Expression::StaticImport(val);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn ref_check(program: &mut Program) -> Result<(), WalkError> {
    let body = &mut program.body;
    let modules = &mut program.modules;

    let mut scope = Scope::new();
    let mut stack = CaptureStack::new();

    let mut lambdas = Lams {
        lambdas: &mut program.lambdas,
        getters: &mut program.getters,
        setters: &mut program.setters,
    };

    for mod_ind in 0..modules.len() {
        let mut module = std::mem::take(&mut modules[mod_ind]);
        walk_module(
            &mut module,
            mod_ind,
            modules,
            &mut lambdas,
            &mut stack,
            &program.strings,
        )?;
        modules[mod_ind] = module;
    }

    for import in &program.imports {
        walk_import(import, &program.modules, &program.strings, &mut scope)?;
    }

    for (ind, Procedure { identifier, .. }) in program.functions.iter().enumerate() {
        if !scope.insert_fn(identifier.data, ind, None) {
            return Err(WalkError {
                identifier: identifier.clone(),
                kind: WalkErrorType::InvalidRedefinition,
            });
        }
    }

    for function in &mut program.functions {
        walk_procedure(function, &mut scope, &mut lambdas, &mut stack)?;
    }

    scope.sub_block(
        |scope, body| walk_ast(body, scope, &mut lambdas, &mut stack),
        body,
    )
}
