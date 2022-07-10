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
