mod opexpr;
mod constructs;

use self::opexpr::{OpExpr, OpExprManager};
use lex::tokens::*;
use location::Located;
use ast::{ParseError, ParseResult};
use ast::parsing::util::take_until;
use ast::node::*;
use std::convert::TryInto;
use util::next_if;
use operators::{op_precedence, Precedence};
use self::constructs::*;
use crate::next_guard;
use ast::parsing::constructs::parse_params;

// Assumes an even groupers
fn expression_list(tokens: Vec<Located<Token>>, in_lam: bool) -> ParseResult<Vec<Expression>> {
    let mut level = 0usize;
    let mut index = 0usize;
    let mut indices = vec![];

    for token in tokens.iter() {
        match token.data {
            Token::OpenGroup(_) => level += 1,
            Token::CloseGroup(_) => {
                level = level
                    .checked_sub(1)
                    .ok_or(ParseError::UnexpectedToken(token.clone()))?;
            }
            Token::Comma if level == 0 => {
                indices.push((std::mem::replace(&mut index, 0usize), token.clone()));
                continue;
            }
            _ => {}
        }
        index += 1;
    }

    let mut tokens = tokens.into_iter();
    let mut expressions = vec![];

    for (i, token) in indices.into_iter() {
        expressions.push(
            parse_expr(
                tokens
                    .by_ref()
                    .enumerate()
                    .take_while(|(index, _)| *index != i)
                    .map(|(_, val)| val)
                    .collect::<Vec<_>>(),
                in_lam
            )
            .map_err(|e| e.neof_or(ParseError::UnexpectedToken(token)))?,
        );
    }

    let remaining = tokens.collect::<Vec<_>>();

    if !remaining.is_empty() {
        expressions.push(parse_expr(remaining, in_lam)?);
    }

    Ok(expressions)
}

type CallbackArg = (Vec<Located<Token>>, Located<Token>);

pub fn parse_expr(tokens: impl IntoIterator<Item = Located<Token>>, in_lam: bool) -> ParseResult<Expression> {
    let mut tokens = tokens.into_iter().peekable();
    let mut op_expr = OpExprManager::new();

    fn list_callback(in_lam: bool) -> impl Fn(CallbackArg) -> ParseResult<Vec<Expression>> {
        move |(v, last)| expression_list(v, in_lam)
            .map_err(|e| e.neof_or(ParseError::UnexpectedToken(last)))
    }

    fn expr_callback(in_lam: bool) -> impl Fn(CallbackArg) -> ParseResult<Expression> {
        move |(v, last)| parse_expr(v, in_lam)
            .map_err(|e| e.neof_or(ParseError::UnexpectedToken(last)))
    }

    while let Some(token) = tokens.next() {
        let mut expr = None;

        let data = token.data.clone();
        let start = token.start.clone();
        match data {
            Token::BinaryOp(binary) => op_expr
                .push(binary.clone())
                .map_err(|_| ParseError::UnexpectedToken(token.clone()))?,
            Token::UnaryOp(unary) => op_expr
                .push(unary.clone())
                .map_err(|_| ParseError::UnexpectedToken(token.clone()))?,
            Token::AssignOp(assign) => op_expr
                .push(assign.clone())
                .map_err(|_| ParseError::UnexpectedToken(token.clone()))?,
            Token::OpenGroup(Grouper::Bracket) => {
                expr = Expression::ArrayCreation(
                    take_until(&mut tokens, Grouper::Bracket).and_then(list_callback(in_lam))?,
                )
                .into();
            }
            Token::OpenGroup(Grouper::Parentheses) => {
                expr = take_until(&mut tokens, Grouper::Parentheses)
                    .and_then(expr_callback(in_lam))?
                    .into();
            }
            Token::Keyword(Keyword::Lam) => {
                let params = parse_params(&mut tokens)?;
                let (body, _) = take_until(&mut tokens, Grouper::Brace)?;

                expr = Expression::Lambda {
                    param_list: params,
                    body: lam_body(body)?,
                }
                .into();
            }
            Token::Hash | Token::MutableHash => {
                expr = next_guard!({ tokens.next() } { Token::OpenGroup(Grouper::Brace) => {
                    let (body, _) = take_until(&mut tokens, Grouper::Brace)?;
                    let hash = parse_hash(body)?;
                    if data == Token::Hash {
                        Expression::Hash(hash)
                    } else {
                        Expression::MutableHash(hash)
                    }
                } })
                .into();
            }
            Token::Keyword(Keyword::Nil) => expr = Expression::Nil.into(),
            Token::Keyword(Keyword::This) if in_lam => expr = Expression::This.into(),
            Token::Bool(b) => expr = Expression::Bool(b).into(),
            Token::String(s) => expr = Expression::String(s).into(),
            Token::Number(n) => expr = Expression::Number(n).into(),
            Token::Identifier(data) => {
                expr = Expression::Identifier(Located {
                    data,
                    start: token.start.clone(),
                    end: token.end.clone(),
                })
                .into()
            }
            Token::Keyword(Keyword::This) => return Err(ParseError::InvalidThisReference(token.start.clone())),
            _ => return Err(ParseError::UnexpectedToken(token)),
        };

        if let Some(mut expression) = expr {
            while let Some(token) = next_if(tokens.by_ref(), |Located { data, .. }| !data.is_op()) {
                expression = match token.data {
                    Token::OpenGroup(Grouper::Parentheses) => Expression::FunctionCall {
                        function: expression.into(),
                        args: take_until(&mut tokens, Grouper::Parentheses)
                            .and_then(list_callback(in_lam))?,
                    },
                    Token::OpenGroup(Grouper::Bracket) => Expression::IndexAccess {
                        item: expression.into(),
                        index: take_until(&mut tokens, Grouper::Bracket)
                            .and_then(expr_callback(in_lam))?
                            .into(),
                    },
                    Token::Period => next_guard!({ tokens.next() } {
                        Token::Keyword(k) => Expression::PropertyAccess {
                            item: expression.into(),
                            property: k.str().into(),
                        },
                        Token::Identifier(property) => Expression::PropertyAccess {
                            item: expression.into(),
                            property,
                        }
                    }),
                    _ => return Err(ParseError::UnexpectedToken(token)),
                }
            }

            op_expr
                .push((expression, start))
                .map_err(|_| ParseError::UnexpectedToken(token))?;
        }
    }

    let mut op_expr: Vec<_> = op_expr.into();

    for index in op_expr
        .iter()
        .enumerate()
        .flat_map(|(i, op_e)| Some(i).filter(|_| op_e.is_unary()))
        .rev()
        .collect::<Vec<_>>()
    {
        if let OpExpr::Unary(unary) = op_expr.remove(index) {
            if let Some(OpExpr::Expr(ref mut expr, _)) = op_expr.get_mut(index) {
                *expr = Expression::Unary {
                    op: unary,
                    expr: std::mem::replace(expr, Expression::Nil).into(),
                };
            } else {
                return Err(ParseError::UnexpectedEOF);
            }
        }
    }

    for precedence in Precedence::ITEMS.iter().rev() {
        for i in op_expr
            .iter()
            .enumerate()
            .flat_map(|(i, op)| match op {
                OpExpr::Binary(b) if *precedence == op_precedence(b) => Some(i),
                _ => None,
            })
            .enumerate()
            .map(|(i, index)| index - i * 2)
            .collect::<Vec<_>>()
        {
            if let (OpExpr::Expr(one, _), OpExpr::Binary(op)) =
                (op_expr.remove(i - 1), op_expr.remove(i - 1))
            {
                if let Some(OpExpr::Expr(ref mut expr, _)) = op_expr.get_mut(i - 1) {
                    *expr = Expression::Binary {
                        op,
                        left: one.into(),
                        right: std::mem::replace(expr, Expression::Nil).into(),
                    }
                } else {
                    return Err(ParseError::UnexpectedEOF);
                }
            }
        }
    }

    for i in op_expr
        .iter()
        .enumerate()
        .flat_map(|(i, v)| Some(i).filter(|_| v.is_assign()))
        .enumerate()
        .map(|(i, index)| index - i * 2)
        .collect::<Vec<_>>()
    {
        if let (OpExpr::Expr(one, start), OpExpr::Assign(op)) =
            (op_expr.remove(i - 1), op_expr.remove(i - 1))
        {
            if let Some(OpExpr::Expr(ref mut expr, _)) = op_expr.get_mut(i - 1) {
                *expr = Expression::Assignment {
                    op,
                    left: one
                        .try_into()
                        .map_err(|_| ParseError::IllegalLeftExpression { start })?,
                    right: std::mem::replace(expr, Expression::Nil).into(),
                }
            } else {
                return Err(ParseError::UnexpectedEOF);
            }
        }
    }

    op_expr
        .pop()
        .filter(|_| op_expr.is_empty())
        .and_then(|expr| {
            if let OpExpr::Expr(e, _) = expr {
                Some(e)
            } else {
                None
            }
        })
        .ok_or(ParseError::UnexpectedEOF)
}