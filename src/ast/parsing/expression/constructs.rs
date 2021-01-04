use ast::node::*;
use ast::{ParseResult, ParseError};
use ast::parsing::{util::*, scope::Scope, ast_level};
use lex::tokens::*;
use location::Located;
use util::next_if;
use operators::Binary;
use crate::next_guard;
use super::parse_expr;

pub fn parse_prop(tokens: impl IntoIterator<Item = Located<Token>>) -> ParseResult<AutoProp> {
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
                    prop.get = LocatedOr::Located(Located { data: s, start, end }).into();
                } else {
                    if next_if(&mut tokens, |t| t.data == Token::BinaryOp(Binary::LogicalOr)).is_none() {
                        if next_if(&mut tokens, |t| t.data == Token::Pipe).is_some() {
                            next_guard!({ tokens.next() } { Token::Pipe => {} });
                        }
                    }
                    prop.get = LocatedOr::Or(next_guard!({ tokens.next() } {
                        Token::OpenGroup(Grouper::Brace) => take_until(&mut tokens, Grouper::Brace)
                            .and_then(|(t, e)| lam_body(t).map_err(|err| {
                                err.neof_or(ParseError::UnexpectedToken(e))
                            }))?
                    })).into();
                }
            } else {
                return Err(ParseError::UnexpectedToken(Located {
                    start, end,
                    data: Token::Keyword(Keyword::Get)
                }));
            },
            Token::Keyword(Keyword::Set) => if prop.set.is_none() {
                if let Some(Located { data: Token::Identifier(s), start, end }) = next_if(&mut tokens, |t| t.data.ident()) {
                    prop.set = LocatedOr::Located(Located { data: s, start, end }).into();
                } else {
                    next_guard!({ tokens.next() } { Token::Pipe => {} });
                    let param = next_guard!({ tokens.next() } { Token::Identifier(s) => s });
                    next_guard!({ tokens.next() } { Token::Pipe => {} });
                    next_guard!({ tokens.next() } { Token::OpenGroup(Grouper::Brace) => {} });
                    prop.set = LocatedOr::Or(SetProp {
                        param,
                        block: take_until(&mut tokens, Grouper::Brace)
                            .and_then(|(t, e)| lam_body(t).map_err(|err| {
                                err.neof_or(ParseError::UnexpectedToken(e))
                            }))?
                    }).into();
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

pub fn parse_hash(tokens: impl IntoIterator<Item = Located<Token>>) -> ParseResult<Hash> {
    let mut tokens = tokens.into_iter().peekable();
    let mut map = Hash::new();
    while tokens.peek().is_some() {
        let key = next_guard!({ tokens.next() } {
            Token::Identifier(s) | Token::String(s) => s,
            Token::Number(n) => n.to_string()
        });
        let value = next_guard!({ tokens.next() } {
            Token::Arrow => match zero_level_preserve(&mut tokens, |t| *t == Token::Comma)? {
                Ok((tokens, _)) | Err(tokens) => parse_expr(tokens, false).map(ObjectValue::Expression)
            },
            Token::OpenGroup(Grouper::Brace) => {
                let (interior, last) = take_until(&mut tokens, Grouper::Brace)?;
                next_if(&mut tokens, |Located { data: t, .. }| *t == Token::Comma);
                parse_prop(interior).map_err(|e| {
                    e.neof_or(ParseError::UnexpectedToken(last))
                }).map(ObjectValue::AutoProp)
            }
        })?;
        map.insert(key, value);
    }
    Ok(map)
}

pub fn lam_body(body: Vec<Located<Token>>) -> ParseResult<LambdaBody> {
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
            parse_expr(body, true)
                .map(Box::from)
                .map(LambdaBody::ImplicitReturn)
        }
    } else {
        ast_level(body, Scope::fn_lam()).map(LambdaBody::Block)
    }
}
