use super::parse_expr;
use crate::next_guard;
use ast::node::*;
use ast::parsing::{ast_level, scope::Scope, util::*};
use ast::{ModuleError, ModuleErrorBody, ParseError, ParseResult};
use lex::{lex, tokens::*};
use location::Located;
use operators::{Assignment, Binary};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use util::remove_file;

pub fn parse_if_block<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    scope: Scope,
    program: &mut Program,
) -> ParseResult<ConditionBodyPair> {
    Ok(ConditionBodyPair {
        condition: zero_level(tokens, |t| *t == Token::OpenGroup(Grouper::Brace))
            .and_then(|(v, _)| parse_expr(v, scope.in_lam, program))?,
        block: take_until(tokens, Grouper::Brace)
            .and_then(|(v, _)| ast_level(v, scope, program))?,
    })
}

pub fn parse_decl<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    mutable: bool,
    scope: Scope,
    program: &mut Program,
) -> ParseResult<Declaration> {
    let mut decls = vec![];
    let mut cont = true;

    while cont {
        let identifier = next_guard!({ tokens.next() } (start, end) {
            Token::Identifier(name) => Located { data: name, start, end }
        });
        decls.push(Declarator {
            identifier,
            captured: false,
            value: next_guard!({ tokens.next() } {
                Token::AssignOp(Assignment::Assign) => {
                    let (v, Located { data: last, .. }) = zero_level(tokens, |d| *d == Token::Semicolon || *d == Token::Comma)?;
                    cont = last == Token::Comma;
                    parse_expr(v, scope.in_lam, program)?
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

pub fn parse_proc<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    public: bool,
    program: &mut Program,
) -> ParseResult<Procedure> {
    let name = next_guard!({ tokens.next() } (start, end) {
        Token::Identifier(i) => Located { data: i, start, end }
    });
    let param_list = parse_params(tokens)?;

    Ok(Procedure {
        identifier: name,
        param_list,
        body: take_until(tokens, Grouper::Brace)
            .and_then(|(v, _)| ast_level(v, Scope::fn_proc(), program))?,
        public,
    })
}

fn module_err(data: ModuleErrorBody, path: Located<PathBuf>) -> ParseError {
    ParseError::ModuleError(ModuleError { path, data })
}

pub fn parse_module(path: &Located<PathBuf>, program: &mut Program) -> ParseResult<CustomModule> {
    let mut dir = path.data.clone();

    let text = fs::read_to_string(&dir)
        .map_err(|_| module_err(ModuleErrorBody::PathNotFound, path.clone()))?;

    let mut tokens = lex(&text)
        .map_err(ModuleErrorBody::LexError)
        .map_err(|e| module_err(e, path.clone()))?
        .into_iter();

    remove_file(&mut dir);

    let mut functions = vec![];
    let mut imports = vec![];

    while let Some(token) = tokens.next() {
        match token.data {
            Token::Keyword(Keyword::Public) => next_guard!({ tokens.next() } {
                Token::Keyword(Keyword::Proc) => functions.push(parse_proc(&mut tokens, true, program)?)
            }),
            Token::Keyword(Keyword::Proc) => {
                functions.push(parse_proc(&mut tokens, true, program)?)
            }
            Token::Keyword(Keyword::Import) => {
                imports.push(parse_import(&mut tokens, dir.as_path(), program)?)
            }
            _ => return Err(ParseError::UnexpectedToken(token)),
        };
    }

    Ok(CustomModule {
        functions,
        imports,
        path: dir,
    })
}

pub fn parse_import<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    path: &Path,
    program: &mut Program,
) -> ParseResult<Import> {
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

                    Module::Custom(match program.has_module(&new_path) {
                        Some(ind) => ind,
                        None => {
                            let module = parse_module(&Located {
                                data: new_path,
                                end: end.clone(),
                                start: start.clone(),
                            }, program)?;
                            program.set_module(module)
                        },
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
