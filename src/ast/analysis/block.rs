use crate::ast::node::{ForLoop, LogicChain};

///
/// Walk functions related to statements and blocks
///
use super::expression::walk_expression;
use super::*;
use ast::node::*;

/// Add declarations to scope
pub fn walk_decl(
    decl: &mut Declaration,
    scope: &mut Scope,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> WalkResult {
    for d in decl.declarations.iter_mut() {
        if let Some(e) = &mut d.value {
            walk_expression(e, scope, lams, cap)?;
        }

        if !scope.insert_var(*d.identifier, decl.mutable) {
            return Err(WalkError {
                data: WalkErrorType::InvalidRedefinition(*d.identifier),
                loc: d.identifier.loc,
            });
        }
    }

    Ok(())
}

/// Walk a block structure
pub fn walk_ast(
    nodes: &mut Block,
    scope: &mut Scope,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> WalkResult {
    for node in nodes.iter_mut() {
        match node {
            // Record the number of declarations in a loop or function up
            // until a control break
            Node::ControlFlow(Located { data: flow, loc }) => match &mut flow.kind {
                BreakType::Return(expr) => {
                    if let Some(e) = expr {
                        walk_expression(e, scope, lams, cap)?;
                    }

                    if let Some(allocs) = scope.fnc_alloc {
                        flow.allocations = allocs;
                    } else {
                        return Err(WalkError {
                            data: WalkErrorType::InvalidReturn,
                            loc: *loc,
                        });
                    }
                }
                BreakType::Break | BreakType::Continue => {
                    if let Some(allocs) = scope.loop_alloc {
                        flow.allocations = allocs;
                    } else {
                        let data = match &flow.kind {
                            BreakType::Break => WalkErrorType::InvalidBreak,
                            BreakType::Continue => WalkErrorType::InvalidContinue,
                            BreakType::Return(_) => unreachable!(),
                        };

                        return Err(WalkError { data, loc: *loc });
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
            Node::LogicChain(LogicChain {
                if_block,
                elseifs,
                else_block,
            }) => {
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
            Node::For(ForLoop {
                declaration,
                condition,
                increment,
                body,
            }) => scope.sub(|scope| {
                if let Some(decl) = declaration {
                    walk_decl(decl, scope, lams, cap)?;
                }

                for expr in condition.iter_mut().chain(increment.iter_mut()) {
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
