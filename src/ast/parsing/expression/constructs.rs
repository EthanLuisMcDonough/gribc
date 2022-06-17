use super::parse_expr;
use crate::next_guard;
use ast::node::*;
use ast::parsing::{ast_level, scope::Scope, util::*, Store};
use ast::{ParseError, ParseResult};
use lex::tokens::*;
use location::Located;
use operators::Binary;
use util::next_if;

pub fn parse_prop(
    tokens: impl IntoIterator<Item = Located<Token>>,
    store: &mut Store,
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
                    prop.get = AutoPropValue::String(Located { data: store.ins_str(s), start, end }).into();
                } else {
                    if next_if(&mut tokens, |t| t.data == Token::BinaryOp(Binary::LogicalOr)).is_none() {
                        if next_if(&mut tokens, |t| t.data == Token::Pipe).is_some() {
                            next_guard!({ tokens.next() } { Token::Pipe => {} });
                        }
                    }

                    let body = next_guard!({ tokens.next() } {
                        Token::OpenGroup(Grouper::Brace) => take_until(&mut tokens, Grouper::Brace)
                            .and_then(|(t, e)| lam_body(t, store).map_err(|err| {
                                err.neof_or(ParseError::UnexpectedToken(e))
                            }))?
                    });

                    let ind = store.add_getter(GetProp::new(body));
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
                    prop.set = AutoPropValue::String(Located { data: store.ins_str(s), start, end }).into();
                } else {
                    next_guard!({ tokens.next() } { Token::Pipe => {} });
                    let param = next_guard!({ tokens.next() } { Token::Identifier(s) => s });
                    next_guard!({ tokens.next() } { Token::Pipe => {} });
                    next_guard!({ tokens.next() } { Token::OpenGroup(Grouper::Brace) => {} });

                    let body = take_until(&mut tokens, Grouper::Brace)
                        .and_then(|(t, e)| lam_body(t, store).map_err(|err| {
                            err.neof_or(ParseError::UnexpectedToken(e))
                        }))?;

                    let param_ind = store.ins_str(param);
                    let ind = store.add_setter(SetProp::new(param_ind, body));
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
    store: &mut Store,
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
                Ok((tokens, _)) | Err(tokens) => parse_expr(tokens, false, store).map(ObjectValue::Expression)
            },
            Token::OpenGroup(Grouper::Brace) => {
                let (interior, last) = take_until(&mut tokens, Grouper::Brace)?;
                next_if(&mut tokens, |Located { data: t, .. }| *t == Token::Comma);
                let prop = parse_prop(interior, store).map_err(|e| {
                    e.neof_or(ParseError::UnexpectedToken(last))
                })?;
                Ok(ObjectValue::AutoProp(prop))
            }
        })?;
        map.insert(store.ins_str(key), value);
    }
    Ok(map)
}

fn leading_stmt(v: &Vec<Located<Token>>) -> bool {
    v.first()
        .filter(|Located { data, .. }| match data {
            Token::Keyword(Keyword::If | Keyword::While | Keyword::For) => true,
            _ => false,
        })
        .is_some()
}

pub fn lam_body(body: Vec<Located<Token>>, store: &mut Store) -> ParseResult<LambdaBody> {
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

    if body.is_empty() {
        Ok(LambdaBody::Block(vec![]))
    } else if semicolons == 0 && !leading_stmt(&body) {
        parse_expr(body, true, store)
            .map(Box::from)
            .map(LambdaBody::ImplicitReturn)
    } else {
        ast_level(body, Scope::fn_lam(), store).map(LambdaBody::Block)
    }
}
