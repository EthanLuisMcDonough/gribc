use super::parse_expr;
use crate::next_guard;
use ast::node::*;
use ast::parsing::{ast_level, scope::Scope, util::*, Store};
use ast::{ModuleError, ModuleErrorBody, ParseError, ParseResult};
use lex::{lex, tokens::*};
use location::Located;
use operators::{Assignment, Binary};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use util::remove_file;

pub fn parse_if_block<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    scope: Scope,
    store: &mut Store,
) -> ParseResult<ConditionBodyPair> {
    Ok(ConditionBodyPair {
        condition: zero_level(tokens, |t| *t == Token::OpenGroup(Grouper::Brace))
            .and_then(|(v, _)| parse_expr(v, scope.in_lam, store))?,
        block: take_until(tokens, Grouper::Brace).and_then(|(v, _)| ast_level(v, scope, store))?,
    })
}

pub fn parse_decl<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    mutable: bool,
    scope: Scope,
    store: &mut Store,
) -> ParseResult<Declaration> {
    let mut decls = vec![];
    let mut cont = true;

    while cont {
        let ident = next_guard!({ tokens.next() } (start, end) {
            Token::Identifier(name) => Located { data: name, start, end }
        });
        decls.push(Declarator {
            identifier: Located {
                data: store.ins_str(ident.data),
                start: ident.start,
                end: ident.end,
            },
            captured: false,
            value: next_guard!({ tokens.next() } {
                Token::AssignOp(Assignment::Assign) => {
                    let (v, Located { data: last, .. }) = zero_level(tokens, |d| *d == Token::Semicolon || *d == Token::Comma)?;
                    cont = last == Token::Comma;
                    parse_expr(v, scope.in_lam, store)?
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
                next_guard!({ token } (start, end) {
                    Token::Identifier(s) => if params.vardic.is_some() {
                        return Err(ParseError::ParamAfterSpread(Located {
                            data: s, start, end
                        }));
                    } else if !params.try_add(store.ins_str(s.clone())) {
                        return Err(ParseError::DuplicateParam(Located {
                            data: s, start, end
                        }));
                    },
                    Token::Spread => next_guard!({ list.next() } (start, end) {
                        Token::Identifier(s) => if params.vardic.is_some() {
                            return Err(ParseError::ParamAfterSpread(Located {
                                data: s, start, end
                            }));
                        } else if store.get_str(&s).map(|i| params.params.contains(&i)).is_some() {
                            return Err(ParseError::DuplicateParam(Located {
                                data: s, start, end
                            }));
                        } else {
                            params.vardic = store.ins_str(s).into();
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
    let (name, start, end) = next_guard!({ tokens.next() } (start, end) {
        Token::Identifier(i) => (i, start, end)
    });
    let ind = store.ins_str(name);
    let param_list = parse_params(tokens, store)?;

    Ok(Procedure {
        identifier: Located {
            data: ind,
            start,
            end,
        },
        param_list,
        body: take_until(tokens, Grouper::Brace)
            .and_then(|(v, _)| ast_level(v, Scope::fn_proc(), store))?,
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

    let mut functions = vec![];
    let mut imports = vec![];

    while let Some(token) = tokens.next() {
        match token.data {
            Token::Keyword(Keyword::Public) => next_guard!({ tokens.next() } {
                Token::Keyword(Keyword::Proc) => functions.push(parse_proc(&mut tokens, true, store)?)
            }),
            Token::Keyword(Keyword::Proc) => functions.push(parse_proc(&mut tokens, true, store)?),
            Token::Keyword(Keyword::Import) => {
                imports.push(parse_import(&mut tokens, dir.as_path(), store)?)
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
    store: &mut Store,
) -> ParseResult<Import> {
    let kind = next_guard!({ tokens.next() } (start, end) {
        Token::BinaryOp(Binary::Mult) => ImportKind::All,
        Token::Identifier(name) => ImportKind::ModuleObject(Located {
            data: store.ins_str(name), start, end
        }),
        Token::Pipe => {
            let (inner, _) = zero_level(tokens, |t| *t == Token::Pipe)?;
            let mut imports = HashMap::with_capacity(inner.len());

            for item in inner {
                if let Token::Identifier(name) = &item.data {
                    imports.insert(store.ins_str(name), (start.clone(), end.clone()));
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
        Token::String(s) => match NativePackage::from_str(&s) {
            Some(package) => Module::Native { package, indices: HashSet::new() },
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

                Module::Custom(match store.get_mod(&new_path) {
                    Some(ind) => *ind,
                    None => {
                        let module = parse_module(&Located {
                            data: new_path,
                            end: end.clone(),
                            start: start.clone(),
                        }, store)?;
                        store.ins_mod(new_path, module)
                    },
                })
            },
        }
    });

    next_guard!({ tokens.next() } {
        Token::Semicolon => {}
    });

    Ok(Import { module, kind })
}
