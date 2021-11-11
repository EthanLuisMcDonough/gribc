use super::parse_expr;
use crate::next_guard;
use ast::node::*;
use ast::parsing::{ast_level, scope::Scope, util::*};
use ast::{ParseError, ParseResult};
use lex::tokens::*;
use location::Located;
use operators::Binary;
use util::next_if;

pub fn parse_prop(
    tokens: impl IntoIterator<Item = Located<Token>>,
    program: &mut Program,
) -> ParseResult<AutoProp> {
    let mut prop = AutoProp::new();

    let mut interior = tokens.into_iter().peekable();
    while interior.peek().is_some() {
        let mut tokens = match zero_level_preserve(&mut interior, |t| *t == Token::Comma)? {
            Ok((tokens, _)) | Err(tokens) => tokens,
        }
        .into_iter()
        .peekable();
        next_guard!({ tokens.next() } (start, end) {
            Token::Keyword(Keyword::Get) => if prop.get.is_none() {
                if let Some(Located { data: Token::Identifier(s), start, end }) = next_if(&mut tokens, |t| t.data.ident()) {
                    prop.get = AutoPropValue::String(Located { data: s, start, end }).into();
                } else {
                    if next_if(&mut tokens, |t| t.data == Token::BinaryOp(Binary::LogicalOr)).is_none() {
                        if next_if(&mut tokens, |t| t.data == Token::Pipe).is_some() {
                            next_guard!({ tokens.next() } { Token::Pipe => {} });
                        }
                    }

                    let ind = program.getters.len();
                    let body = next_guard!({ tokens.next() } {
                        Token::OpenGroup(Grouper::Brace) => take_until(&mut tokens, Grouper::Brace)
                            .and_then(|(t, e)| lam_body(t, program).map_err(|err| {
                                err.neof_or(ParseError::UnexpectedToken(e))
                            }))?
                    });

                    program.getters.push(GetProp::new(body));
                    prop.get = AutoPropValue::Lambda(ind).into();
                }
            } else {
                return Err(ParseError::UnexpectedToken(Located {
                    start, end,
                    data: Token::Keyword(Keyword::Get)
                }));
            },
            Token::Keyword(Keyword::Set) => if prop.set.is_none() {
                if let Some(Located { data: Token::Identifier(s), start, end }) = next_if(&mut tokens, |t| t.data.ident()) {
                    prop.set = AutoPropValue::String(Located { data: s, start, end }).into();
                } else {
                    next_guard!({ tokens.next() } { Token::Pipe => {} });
                    let param = next_guard!({ tokens.next() } { Token::Identifier(s) => s });
                    next_guard!({ tokens.next() } { Token::Pipe => {} });
                    next_guard!({ tokens.next() } { Token::OpenGroup(Grouper::Brace) => {} });

                    let ind = program.setters.len();
                    let body = take_until(&mut tokens, Grouper::Brace)
                        .and_then(|(t, e)| lam_body(t, program).map_err(|err| {
                            err.neof_or(ParseError::UnexpectedToken(e))
                        }))?;

                    program.setters.push(SetProp::new(param, body));
                    prop.set = AutoPropValue::Lambda(ind).into();
                }
            } else {
                return Err(ParseError::UnexpectedToken(Located {
                    start, end,
                    data: Token::Keyword(Keyword::Set)
                }));
            }
        });
    }

    Ok(prop)
}

pub fn parse_hash(
    tokens: impl IntoIterator<Item = Located<Token>>,
    program: &mut Program,
) -> ParseResult<Hash> {
    let mut tokens = tokens.into_iter().peekable();
    let mut map = Hash::new();
    while tokens.peek().is_some() {
        let key = next_guard!({ tokens.next() } {
            Token::Identifier(s) | Token::String(s) => s,
            Token::Number(n) => n.to_string()
        });
        let value = next_guard!({ tokens.next() } {
            Token::Arrow => match zero_level_preserve(&mut tokens, |t| *t == Token::Comma)? {
                Ok((tokens, _)) | Err(tokens) => parse_expr(tokens, false, program).map(ObjectValue::Expression)
            },
            Token::OpenGroup(Grouper::Brace) => {
                let (interior, last) = take_until(&mut tokens, Grouper::Brace)?;
                next_if(&mut tokens, |Located { data: t, .. }| *t == Token::Comma);
                let prop = parse_prop(interior, program).map_err(|e| {
                    e.neof_or(ParseError::UnexpectedToken(last))
                })?;
                Ok(ObjectValue::AutoProp(prop))
            }
        })?;
        map.insert(key, value);
    }
    Ok(map)
}

pub fn lam_body(body: Vec<Located<Token>>, program: &mut Program) -> ParseResult<LambdaBody> {
    let mut level = 0;
    let mut semicolons = 0;
    for token in &body {
        match token.data {
            Token::OpenGroup(_) => level += 1,
            Token::CloseGroup(_) => level -= 1,
            Token::Semicolon if level == 0 => semicolons += 1,
            _ => {}
        }
    }

    if semicolons == 0 {
        if body.is_empty() {
            Ok(LambdaBody::Block(vec![]))
        } else {
            parse_expr(body, true, program)
                .map(Box::from)
                .map(LambdaBody::ImplicitReturn)
        }
    } else {
        ast_level(body, Scope::fn_lam(), program).map(LambdaBody::Block)
    }
}
