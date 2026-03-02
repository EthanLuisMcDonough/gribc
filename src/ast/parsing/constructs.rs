use super::parse_expr;
use crate::next_guard;
use ast::node::*;
use ast::parsing::{ast_level, util::*, Store};
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
    store: &mut Store,
) -> ParseResult<ConditionBodyPair> {
    Ok(ConditionBodyPair {
        condition: zero_level(tokens, |t| *t == Token::OpenGroup(Grouper::Brace))
            .and_then(|(v, _)| parse_expr(v, store))?,
        block: take_until(tokens, Grouper::Brace).and_then(|(v, _)| ast_level(v, store))?,
    })
}

pub fn parse_decl<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    mutable: bool,
    store: &mut Store,
) -> ParseResult<Declaration> {
    let mut decls = vec![];
    let mut cont = true;

    while cont {
        let ident = next_guard!({ tokens.next() } (loc) {
            Token::Identifier(name) => Located { data: name, loc }
        });
        decls.push(Declarator {
            identifier: Located {
                data: store.ins_ident(ident.data),
                loc: ident.loc,
            },
            captured: false,
            value: next_guard!({ tokens.next() } {
                Token::AssignOp(Assignment::Assign) => {
                    let (v, Located { data: last, .. }) = zero_level(tokens, |d| *d == Token::Semicolon || *d == Token::Comma)?;
                    cont = last == Token::Comma;
                    Some(parse_expr(v, store)?)
                },
                Token::Semicolon => {
                    cont = false;
                    None
                },
                Token::Comma => None
            }),
        });
    }

    Ok(Declaration {
        declarations: decls,
        mutable,
    })
}

pub fn parse_params<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    store: &mut Store,
) -> ParseResult<Parameters> {
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
                next_guard!({ token } (loc) {
                    Token::Identifier(s) => if params.vardic.is_some() {
                        return Err(ParseError::ParamAfterSpread(Located {
                            data: s, loc
                        }));
                    } else if !params.try_add(Located { data: store.ins_ident(s.clone()), loc }) {
                        return Err(ParseError::DuplicateParam(Located {
                            data: s, loc
                        }));
                    },
                    Token::Spread => next_guard!({ list.next() } (loc) {
                        Token::Identifier(s) => if params.vardic.is_some() {
                            return Err(ParseError::ParamAfterSpread(Located {
                                data: s, loc
                            }));
                        } else if store.get_ident(&s).filter(|&i| params.contains(i)).is_some() {
                            return Err(ParseError::DuplicateParam(Located {
                                data: s, loc
                            }));
                        } else {
                            params.vardic = Param {
                                name: Located { data: store.ins_ident(s), loc },
                                captured: false,
                            }.into();
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
    store: &mut Store,
) -> ParseResult<Procedure> {
    let (name, loc) = next_guard!({ tokens.next() } (span) {
        Token::Identifier(i) => (i, span)
    });
    let ind = store.ins_ident(name);
    let param_list = parse_params(tokens, store)?;

    Ok(Procedure {
        identifier: Located { data: ind, loc },
        param_list,
        body: take_until(tokens, Grouper::Brace).and_then(|(v, _)| ast_level(v, store))?,
        public,
    })
}

fn module_err(data: ModuleErrorBody, path: Located<PathBuf>) -> ParseError {
    ParseError::ModuleError(ModuleError { path, data })
}

pub fn parse_module(path: &Located<PathBuf>, store: &mut Store) -> ParseResult<CustomModule> {
    let mut dir = path.data.clone();

    let text = fs::read_to_string(&dir)
        .map_err(|_| module_err(ModuleErrorBody::PathNotFound, path.clone()))?;

    let mut tokens = lex(&text)
        .map_err(ModuleErrorBody::LexError)
        .map_err(|e| module_err(e, path.clone()))?
        .into_iter();

    remove_file(&mut dir);

    let mut procedures = Procedures::new();
    let mut imports = Imports::new();

    while let Some(token) = tokens.next() {
        match token.data {
            Token::Keyword(Keyword::Public) => next_guard!({ tokens.next() } {
                Token::Keyword(Keyword::Proc) => procedures.push(parse_proc(&mut tokens, true, store)?)
            }),
            Token::Keyword(Keyword::Proc) => procedures.push(parse_proc(&mut tokens, true, store)?),
            Token::Keyword(Keyword::Import) => {
                imports.push(parse_import(&mut tokens, dir.as_path(), store)?)
            }
            _ => return Err(ParseError::UnexpectedToken(token)),
        };
    }

    let proc_len = procedures.len();
    Ok(CustomModule {
        procedures,
        imports,
        path: dir,
        lookup: HashMap::with_capacity(proc_len),
    })
}

pub fn parse_import<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    path: &Path,
    store: &mut Store,
) -> ParseResult<Import> {
    let kind = next_guard!({ tokens.next() } (loc) {
        Token::BinaryOp(Binary::Mult) => ImportKind::All,
        Token::Identifier(name) => ImportKind::ModuleObject(Located {
            data: store.ins_ident(name), loc
        }),
        Token::Pipe => {
            let (inner, _) = zero_level(tokens, |t| *t == Token::Pipe)?;
            let mut imports = Vec::with_capacity(inner.len());

            for item in inner {
                if let Located { loc, data: Token::Identifier(s) } = item {
                    let data = store.ins_ident(s);
                    imports.push(Located {
                        data, loc
                    });
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

    let module = next_guard!({ tokens.next() } (loc) {
        Token::String(s) => {
            let mod_ind = match NativePackage::from_str(&s) {
                Some(package) => {
                    Module::Native(package)
                },
                None => {
                    let new_buf = path.join(&s);
                    let new_path = new_buf.as_path()
                        .canonicalize()
                        .map_err(|_| module_err(
                            ModuleErrorBody::CantResolveImport,
                            Located {
                                data: new_buf, loc
                            }))?;

                    Module::Custom(match store.get_mod(&new_path) {
                        Some(ind) => ind,
                        None => {
                            let module = parse_module(&Located {
                                data: new_path.clone(), loc,
                            }, store)?;
                            store.ins_mod(new_path, module)
                        },
                    })
                },
            };
            Located { data: mod_ind, loc }
        }
    });

    next_guard!({ tokens.next() } {
        Token::Semicolon => {}
    });

    Ok(Import { module, kind })
}
