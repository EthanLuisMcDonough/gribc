use ast::{ParseError, ParseResult, ModuleErrorBody, ModuleError};
use ast::parsing::{util::*, scope::Scope, ast_level};
use ast::node::*;
use operators::{Assignment, Binary};
use location::{Located};
use lex::{tokens::*, lex};
use util::remove_file;
use std::{fs, collections::HashMap, path::Path};
use super::parse_expr;
use crate::next_guard;

pub fn parse_if_block<'a, T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    scope: Scope<'a>
) -> ParseResult<ConditionBodyPair> {
    Ok(ConditionBodyPair {
        condition: zero_level(tokens, |t| *t == Token::OpenGroup(Grouper::Brace))
            .and_then(|(v, _)| parse_expr(v, scope.in_lam))?,
        block: take_until(tokens, Grouper::Brace)
            .and_then(|(v, _)| ast_level(v, scope))?,
    })
}

pub fn parse_decl<'a, T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    mutable: bool,
    scope: Scope<'a>
) -> ParseResult<Declaration> {
    let mut decls = vec![];
    let mut cont = true;

    while cont {
        let identifier = next_guard!({ tokens.next() } (start, end) {
            Token::Identifier(name) => Located { data: name, start, end }
        });
        decls.push(Declarator {
            identifier,
            value: next_guard!({ tokens.next() } {
                Token::AssignOp(Assignment::Assign) => {
                    let (v, Located { data: last, .. }) = zero_level(tokens, |d| *d == Token::Semicolon || *d == Token::Comma)?;
                    cont = last == Token::Comma;
                    parse_expr(v, scope.in_lam)?
                },
                Token::Semicolon => {
                    cont = false;
                    Expression::Nil
                },
                Token::Comma => Expression::Nil
            }),
        });
    }

    Ok(Declaration {
        declarations: decls,
        mutable,
    })
}

pub fn parse_params<T: Iterator<Item = Located<Token>>>(tokens: &mut T) -> ParseResult<Parameters> {
    let mut params = Parameters::new();
    Ok(next_guard!({ tokens.next() } {
        Token::OpenGroup(Grouper::Brace) => params,
        Token::BinaryOp(Binary::LogicalOr) => {
            next_guard!({ tokens.next() } { Token::OpenGroup(Grouper::Brace) => {} });
            params
        },
        Token::Pipe => {
            let (list, _) = zero_level(tokens, |d| *d == Token::Pipe)?;
            let mut list = list.into_iter();

            next_guard!({ tokens.next() } { Token::OpenGroup(Grouper::Brace) => {} });

            while let Some(token) = list.next() {
                next_guard!({ token } (start, end) {
                    Token::Identifier(s) => if params.vardic.is_some() {
                        return Err(ParseError::ParamAfterSpread(Located {
                            data: s, start, end
                        }));
                    } else if !params.params.insert(s.clone()) {
                        return Err(ParseError::DuplicateParam(Located {
                            data: s, start, end
                        }));
                    },
                    Token::Spread => next_guard!({ list.next() } (start, end) {
                        Token::Identifier(s) => if params.vardic.is_some() {
                            return Err(ParseError::ParamAfterSpread(Located {
                                data: s, start, end
                            }));
                        } else if params.params.contains(&s) {
                            return Err(ParseError::DuplicateParam(Located {
                                data: s, start, end
                            }));
                        } else {
                            params.vardic = s.to_owned().into();
                        }
                    })
                });
            }
            params
        }
    }))
}

pub fn parse_proc<T: Iterator<Item = Located<Token>>>(tokens: &mut T, public: bool) -> ParseResult<Procedure> {
    let name = next_guard!({ tokens.next() } (start, end) {
        Token::Identifier(i) => Located { data: i, start, end }
    });
    let param_list = parse_params(tokens)?;

    Ok(Procedure {
        identifier: name,
        param_list,
        body: take_until(tokens, Grouper::Brace).and_then(|(v, _)| ast_level(v, Scope::fn_proc()))?,
        public
    })
}

fn module_err(data: ModuleErrorBody, path: Located<String>) -> ParseError {
    ParseError::ModuleError(ModuleError { path, data })
}

fn parse_module<'a>(path: Located<String>, scope: Scope<'a>) -> ParseResult<CustomModule> {
    let mut dir = scope.path.map(|p| p.join(&path.data))
        .unwrap_or(Path::new(&path.data).to_path_buf());
    
    let text = fs::read_to_string(dir.as_path())
        .map_err(|_| module_err(ModuleErrorBody::PathNotFound, path.clone()))?;
    
    remove_file(&mut dir);

    let mut tokens = lex(&text).map_err(ModuleErrorBody::LexError)
        .map_err(|e| module_err(e, path.clone()))?.into_iter();

    let mut functions = vec![];
    let mut imports = vec![];

    while let Some(token) = tokens.next() {
        match token.data { 
            Token::Keyword(Keyword::Public) => next_guard!({ tokens.next() } { 
                Token::Keyword(Keyword::Proc) => functions.push(parse_proc(&mut tokens, true)?)
            }),
            Token::Keyword(Keyword::Proc) => functions.push(parse_proc(&mut tokens, true)?),
            Token::Keyword(Keyword::Import) => imports.push(parse_import(&mut tokens, 
                Scope::new(dir.as_path()))?),
            _ => return Err(ParseError::UnexpectedToken(token)),
        };
    }

    Ok(CustomModule { functions, imports })
}

pub fn parse_import<'a, T: Iterator<Item = Located<Token>>>(tokens: &mut T, scope: Scope<'a>) -> ParseResult<Import> {
    let kind = next_guard!({ tokens.next() } (start, end) {
        Token::BinaryOp(Binary::Mult) => ImportKind::All,
        Token::Identifier(name) => ImportKind::ModuleObject(Located {
            data: name, start, end
        }),
        Token::Pipe => {
            let (inner, _) = zero_level(tokens, |t| *t == Token::Pipe)?;
            let mut imports = HashMap::with_capacity(inner.len());

            for item in inner {
                if let Token::Identifier(name) = &item.data {
                    if imports.contains_key(name) {
                        return Err(ParseError::DuplicateParam(Located {
                            data: name.clone(), 
                            start: item.start.clone(), 
                            end: item.end.clone()
                        }));
                    }

                    imports.insert(name.clone(), (start.clone(), end.clone()));
                } else {
                    return Err(ParseError::UnexpectedToken(item));
                }
            }

            ImportKind::List(imports)
        }
    });

    next_guard!({ tokens.next() } {
        Token::Keyword(Keyword::From) => {}
    });
    
    let module = next_guard!({ tokens.next() } (start, end) {
        Token::String(s) => {
            match NativePackage::from_str(&s) {
                Some(package) => Module::Native(package),
                None => Module::Custom(parse_module(Located { 
                    data: s, start, end 
                }, scope)?),
            }
        }
    });

    next_guard!({ tokens.next() } {
        Token::Semicolon => {}
    });

    Ok(Import { module, kind })
}
