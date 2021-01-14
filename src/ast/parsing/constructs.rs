use ast::{ParseError, ParseResult, ModuleErrorBody, ModuleError};
use ast::parsing::{util::*, scope::Scope, ast_level};
use ast::node::*;
use operators::{Assignment, Binary};
use location::{Located};
use lex::{tokens::*, lex};
use util::remove_file;
use std::{fs, collections::HashMap, path::{PathBuf, Path}};
use super::parse_expr;
use crate::next_guard;

pub fn parse_if_block<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    scope: Scope
) -> ParseResult<ConditionBodyPair> {
    Ok(ConditionBodyPair {
        condition: zero_level(tokens, |t| *t == Token::OpenGroup(Grouper::Brace))
            .and_then(|(v, _)| parse_expr(v, scope.in_lam))?,
        block: take_until(tokens, Grouper::Brace)
            .and_then(|(v, _)| ast_level(v, scope))?,
    })
}

pub fn parse_decl<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    mutable: bool,
    scope: Scope
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

fn module_err(data: ModuleErrorBody, path: Located<PathBuf>) -> ParseError {
    ParseError::ModuleError(ModuleError { path, data })
}

pub fn parse_module(path: &Located<PathBuf>) -> ParseResult<CustomModule> {
    let mut dir = path.data.clone();

    let text = fs::read_to_string(&dir)
        .map_err(|_| module_err(ModuleErrorBody::PathNotFound, path.clone()))?;
    
    let mut tokens = lex(&text).map_err(ModuleErrorBody::LexError)
        .map_err(|e| module_err(e, path.clone()))?.into_iter();
    
    remove_file(&mut dir);

    let mut functions = vec![];
    let mut imports = vec![];

    while let Some(token) = tokens.next() {
        match token.data { 
            Token::Keyword(Keyword::Public) => next_guard!({ tokens.next() } { 
                Token::Keyword(Keyword::Proc) => functions.push(parse_proc(&mut tokens, true)?)
            }),
            Token::Keyword(Keyword::Proc) => functions.push(parse_proc(&mut tokens, true)?),
            Token::Keyword(Keyword::Import) => imports.push(parse_import(&mut tokens, 
                dir.as_path())?),
            _ => return Err(ParseError::UnexpectedToken(token)),
        };
    }

    Ok(CustomModule { functions, imports })
}

pub fn parse_import<T: Iterator<Item = Located<Token>>>(tokens: &mut T, path: &Path) -> ParseResult<Import> {
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
                None => {
                    let new_buf = path.join(&s);
                    let new_path = new_buf.as_path()
                        .canonicalize()
                        .map_err(|_| module_err(
                            ModuleErrorBody::CantResolveImport, 
                            Located {
                                data: new_buf,
                                end: end.clone(), 
                                start: start.clone(),
                            }))?;
                    
                    Module::Custom(Located {
                        data: new_path,
                        end: end.clone(), 
                        start: start.clone(),
                    })
                },
            }
        }
    });

    next_guard!({ tokens.next() } {
        Token::Semicolon => {}
    });

    Ok(Import { module, kind })
}
