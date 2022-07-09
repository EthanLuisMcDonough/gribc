mod scope;

use self::scope::*;
use ast::node::*;
use location::{Located, Location};
use std::collections::HashSet;
use std::mem;

pub type WalkResult = Result<(), WalkError>;
type Lambdas = Vec<Lambda>;
type Strings<'a> = &'a Vec<String>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Copy)]
pub enum WalkErrorType {
    ImmutableModification(usize),
    IdentifierNotFound(usize),
    InvalidRedefinition(usize),
    InvalidBreak,
    InvalidReturn,
    InvalidContinue,
    InvalidThis,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WalkError {
    start: Location,
    end: Location,
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
                        start: located.start.clone(),
                        end: located.end.clone(),
                        kind: WalkErrorType::InvalidRedefinition(name),
                    });
                }
                inserted.insert(name);

                let contains = match &import.module {
                    Module::Custom(ind) => modules[*ind].get_function(name).is_some(),
                    Module::Native(pkg) => pkg.fn_from_str(&*strings[name]).is_some(),
                };

                if !contains {
                    return Err(WalkError {
                        start: located.start.clone(),
                        end: located.end.clone(),
                        kind: WalkErrorType::IdentifierNotFound(name),
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
                start: identifier.start.clone(),
                end: identifier.end.clone(),
                kind: WalkErrorType::InvalidRedefinition(identifier.data),
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
            let start = d.identifier.start.clone();
            let end = d.identifier.end.clone();
            let name = d.identifier.data;
            return Err(WalkError {
                start,
                end,
                kind: WalkErrorType::InvalidRedefinition(name),
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
            Node::ControlFlow(flow) => match &mut flow.kind {
                BreakType::Return(expr) => {
                    walk_expression(expr, scope, lams, cap)?;
                    if let Some(allocs) = scope.fnc_alloc {
                        flow.allocations = allocs;
                    } else {
                        return Err(WalkError {
                            kind: WalkErrorType::InvalidReturn,
                            start: flow.start.clone(),
                            end: flow.end.clone(),
                        });
                    }
                }
                BreakType::Break | BreakType::Continue => {
                    if let Some(allocs) = scope.loop_alloc {
                        flow.allocations = allocs;
                    } else {
                        let kind = match &flow.kind {
                            BreakType::Break => WalkErrorType::InvalidBreak,
                            BreakType::Continue => WalkErrorType::InvalidContinue,
                            BreakType::Return(_) => panic!("Unreachable"),
                        };

                        let start = flow.start.clone();
                        let end = flow.end.clone();

                        return Err(WalkError { kind, start, end });
                    }
                }
            },
            Node::Expression(expression) => walk_expression(expression, scope, lams, cap)?,
            Node::Block(nodes) => {
                scope.sub_block(|sub, nodes| walk_ast(nodes, sub, lams, cap), nodes)?;
            }
            Node::While(ConditionBodyPair { condition, block }) => {
                walk_expression(condition, scope, lams, cap)?;
                scope.sub_loop(|sub, block| walk_ast(block, sub, lams, cap), block)?;
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
            } => scope.sub(|scope| {
                if let Some(decl) = declaration {
                    walk_decl(decl, scope, lams, cap)?;
                }
                if let Some(expr) = condition {
                    walk_expression(expr, scope, lams, cap)?;
                }
                if let Some(expr) = increment {
                    walk_expression(expr, scope, lams, cap)?;
                }
                scope.sub_loop(|scope, body| walk_ast(body, scope, lams, cap), body)
            })?,
        }
    }

    if scope.in_first_pass() {
        nodes.allocations = scope.local;
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
                Assignable::Offset(_) => {}
                Assignable::Identifier(i) => {
                    let s = i.data;

                    if scope.in_first_pass() {
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
                            Some(AutoPropValue::Lambda(ind)) if scope.in_first_pass() => scope
                                .sub_with(SubState::InFunc, |scope| {
                                    let mut second = scope.clone();
                                    cap.add(scope.level);

                                    scope.lam_pass = Some(LamPass::First);
                                    second.lam_pass = Some(LamPass::Second);

                                    let mut get = mem::take(&mut lams.getters[ind]);
                                    walk_lambda_block(&mut get.block, scope, lams, cap)?;
                                    get.capture = cap.pop(&mut second);

                                    walk_lambda_block(&mut get.block, &mut second, lams, cap)?;
                                    lams.getters[ind] = get;
                                    Ok(())
                                })?,
                            Some(AutoPropValue::String(ident)) => {
                                if scope.in_first_pass() && !scope.prop_check(ident.data) {
                                    return Err(WalkError {
                                        start: ident.start,
                                        end: ident.end,
                                        kind: WalkErrorType::IdentifierNotFound(ident.data),
                                    });
                                }
                                if scope.in_second_pass() {
                                    let op = scope.runtime_value(ident.data);
                                    auto.get = op.map(AutoPropValue::Value);
                                }
                            }
                            _ => {}
                        }
                        match auto.set.clone() {
                            Some(AutoPropValue::Lambda(ind)) if scope.in_first_pass() => scope
                                .sub_with(SubState::InFunc, |scope| {
                                    let mut second = scope.clone();
                                    cap.add(scope.level);

                                    scope.lam_pass = Some(LamPass::First);
                                    second.lam_pass = Some(LamPass::Second);

                                    let mut set = mem::take(&mut lams.setters[ind]);
                                    scope.insert_mut(set.param);
                                    walk_lambda_block(&mut set.block, scope, lams, cap)?;

                                    set.param_captured = scope.is_captured(set.param);
                                    set.capture = cap.pop(&mut second);

                                    second.insert_mut(set.param);
                                    walk_lambda_block(&mut set.block, &mut second, lams, cap)?;

                                    lams.setters[ind] = set;
                                    Ok(())
                                })?,
                            Some(AutoPropValue::String(ident)) => {
                                if scope.in_first_pass() {
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
                                }
                                if scope.in_second_pass() {
                                    let op = scope.runtime_value(ident.data);
                                    auto.set = op.map(AutoPropValue::Value);
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
            scope.sub_with(SubState::InFunc, |scope| {
                let mut lambda = mem::take(&mut lams.lambdas[*ind]);
                let body = &mut lambda.body;

                let mut second = scope.clone();
                cap.add(scope.level);

                scope.lam_pass = Some(LamPass::First);
                second.lam_pass = Some(LamPass::Second);

                scope.add_params(&lambda.param_list);
                walk_lambda_block(body, scope, lams, cap)?;
                scope.check_params(&mut lambda.param_list);

                lambda.captured = cap.pop(&mut second);
                second.add_params(&lambda.param_list);

                walk_lambda_block(body, &mut second, lams, cap)?;
                lams.lambdas[*ind] = lambda;

                Ok(())
            })?;
        }
        Expression::Identifier(identifier) => {
            if scope.in_first_pass() && !scope.has(identifier.data, cap) {
                return Err(WalkError {
                    start: identifier.start.clone(),
                    end: identifier.end.clone(),
                    kind: WalkErrorType::IdentifierNotFound(identifier.data),
                });
            }
            if scope.in_second_pass() {
                if let Some(val) = scope.runtime_value(identifier.data) {
                    *expression = Expression::Value(val);
                }
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
                start: identifier.start.clone(),
                end: identifier.end.clone(),
                kind: WalkErrorType::InvalidRedefinition(identifier.data),
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
