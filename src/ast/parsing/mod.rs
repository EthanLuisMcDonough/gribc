mod constructs;
mod expression;
mod scope;
mod util;

use self::constructs::*;
use self::expression::parse_expr;
use self::scope::Scope;
use self::util::*;
use ast::node::*;
use lex::{tokens::*, LexError};
use location::{Located, Location};
use std::{
    convert::AsRef,
    iter::Peekable,
    path::{Path, PathBuf},
};
use util::{next_if, remove_file};

pub type ParseResult<T> = Result<T, ParseError>;

#[macro_export]
macro_rules! next_guard {
    ({ $next:expr } ( $start_bind:ident, $end_bind:ident ) { $( $( $p:pat )|* => $b:expr ),* } ) => {
        match $next.into() {
            $($(
                Some(Located {
                    data: $p,
                    start: $start_bind,
                    end: $end_bind,
                }) => $b,
            )*)*
            Some(t) => return Err(ParseError::UnexpectedToken(t)),
            None => return Err(ParseError::UnexpectedEOF),
        };
    };
    ({ $next:expr } { $( $token:tt )* }) => {
        next_guard!({ $next } (_s, _e) { $( $token )* })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ParseError {
    UnexpectedEOF,
    UnexpectedToken(Located<Token>),

    IllegalBreak(Location),
    IllegalContinue(Location),
    IllegalReturn(Location),
    IllegallyScopedImport(Location),
    IllegalExpression(Location),
    IllegalLeftExpression(Location),
    InvalidThisReference(Location),

    FunctionNotAtTopLevel(Location),
    DuplicateParam(Located<String>),
    ParamAfterSpread(Located<String>),

    ModuleError(ModuleError),
    MisplacedImport(Location),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ModuleErrorBody {
    LexError(LexError),
    ParseError(Box<ParseError>),
    PathNotFound,
    CantResolveImport,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ModuleError {
    pub path: Located<PathBuf>,
    pub data: ModuleErrorBody,
}

impl ParseError {
    fn neof_or(self, o: Self) -> Self {
        Some(self)
            .filter(|e| *e != ParseError::UnexpectedEOF)
            .unwrap_or(o)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ParseError {}

fn next_construct<T: Iterator<Item = Located<Token>>>(
    token: Located<Token>,
    tokens: &mut Peekable<T>,
    scope: Scope,
    program: &mut Program,
) -> ParseResult<Node> {
    Ok(match token.data {
        Token::OpenGroup(Grouper::Bracket) => Node::Block(
            take_until(tokens, Grouper::Brace).and_then(|(v, _)| ast_level(v, scope, program))?,
        ),

        Token::Keyword(Keyword::While) => {
            parse_if_block(tokens, scope.with_loop(true), program).map(Node::While)?
        }
        Token::Keyword(Keyword::If) => {
            let if_block = parse_if_block(tokens, scope, program)?;
            let mut elseifs = vec![];
            let mut else_block = None;

            while let Some(_) = next_if(tokens, |Located { data: t, .. }| {
                *t == Token::Keyword(Keyword::Else) && else_block.is_none()
            }) {
                next_guard!({ tokens.next() } {
                    Token::OpenGroup(Grouper::Brace) => else_block = take_until(tokens, Grouper::Brace)
                        .and_then(|(v, _)| ast_level(v, scope, program))?.into(),
                    Token::Keyword(Keyword::If) => elseifs.push(parse_if_block(tokens, scope, program)?)
                });
            }

            Node::LogicChain {
                if_block,
                elseifs,
                else_block,
            }
        }

        Token::Keyword(Keyword::Break) if !scope.in_loop => {
            return Err(ParseError::IllegalBreak(token.start))
        }
        Token::Keyword(Keyword::Continue) if !scope.in_loop => {
            return Err(ParseError::IllegalContinue(token.start))
        }

        Token::Keyword(Keyword::Break) => next_guard!({ tokens.next() }
            { Token::Semicolon => Node::Break }),
        Token::Keyword(Keyword::Continue) => next_guard!({ tokens.next() }
            { Token::Semicolon => Node::Continue }),
        Token::Keyword(Keyword::Return) if !scope.in_proc => {
            return Err(ParseError::IllegalReturn(token.start))
        }
        Token::Keyword(Keyword::Return) => {
            let (tokens, _) = zero_level(tokens, |t| *t == Token::Semicolon)?;
            Node::Return(if tokens.is_empty() {
                Expression::Nil
            } else {
                parse_expr(tokens, scope.in_lam, program)?
            })
        }

        Token::Keyword(Keyword::Decl) => {
            Node::Declaration(parse_decl(tokens, true, scope, program)?)
        }
        Token::Keyword(Keyword::Im) => {
            Node::Declaration(parse_decl(tokens, false, scope, program)?)
        }

        Token::Keyword(Keyword::Proc) | Token::Keyword(Keyword::Public) => {
            return Err(ParseError::FunctionNotAtTopLevel(token.start.clone()))
        }
        Token::Keyword(Keyword::Import) => {
            return Err(ParseError::MisplacedImport(token.start.clone()))
        }

        Token::Keyword(Keyword::For) => {
            let declaration = next_guard!({ tokens.next() } {
                Token::Keyword(Keyword::Decl) => parse_decl(tokens, true, scope, program)?.into(),
                Token::Keyword(Keyword::Im) => parse_decl(tokens, false, scope, program)?.into(),
                Token::Semicolon => None
            });

            let (t, _) = zero_level(tokens, |d| *d == Token::Semicolon)?;
            let condition = if t.is_empty() {
                None
            } else {
                parse_expr(t, scope.in_lam, program)?.into()
            };

            let (t, _) = zero_level(tokens, |d| *d == Token::OpenGroup(Grouper::Brace))?;
            let increment = if t.is_empty() {
                None
            } else {
                parse_expr(t, scope.in_lam, program)?.into()
            };

            let body = take_until(tokens, Grouper::Brace)
                .and_then(|(v, _)| ast_level(v, scope.with_loop(true), program))?;

            Node::For {
                declaration,
                condition,
                increment,
                body,
            }
        }

        Token::Semicolon => {
            return Err(ParseError::UnexpectedToken(Located {
                data: Token::Semicolon,
                start: token.start.clone(),
                end: token.end.clone(),
            }))
        }

        _ => {
            let loc = token.start.clone();
            let (mut tokens, semi) = zero_level(tokens, |t| *t == Token::Semicolon)?;
            tokens.insert(0, token);

            let expr = parse_expr(tokens, scope.in_lam, program)
                .map_err(|e| e.neof_or(ParseError::UnexpectedToken(semi)))?;

            if !expr.is_statement() {
                return Err(ParseError::IllegalExpression(loc));
            }

            Node::Expression(expr)
        }
    })
}

fn top_level(
    tokens: impl IntoIterator<Item = Located<Token>>,
    path: &Path,
) -> ParseResult<Program> {
    fn is_import(t: &Located<Token>) -> bool {
        t.data == Token::Keyword(Keyword::Import)
    }

    let mut program = Program::new();
    let mut tokens = tokens.into_iter().peekable();

    while next_if(&mut tokens, is_import).is_some() {
        let import = parse_import(&mut tokens, path, &mut program)?;

        /*if let Module::Custom(buffer) = &import.module {
            let path = buffer.data.as_path();

            if !program.has_module(path) {
                let module = parse_module(buffer)?;
                program.set_module(buffer.data.clone(), module);
            }
        }*/

        program.imports.push(import);
    }

    while let Some(token) = tokens.next() {
        match token.data {
            Token::Keyword(Keyword::Proc) => {
                let proc = parse_proc(&mut tokens, false, &mut program)?;
                program.functions.push(proc);
            }
            Token::Keyword(Keyword::Public) => next_guard!({ tokens.next() } {
                Token::Keyword(Keyword::Proc) => {
                    let proc = parse_proc(&mut tokens, true, &mut program)?;
                    program.functions.push(proc);
                }
            }),
            _ => {
                let construct =
                    next_construct(token.clone(), &mut tokens, Scope::new(), &mut program)?;
                program.body.push(construct);
            }
        };
    }

    Ok(program)
}

fn ast_level(
    tokens: impl IntoIterator<Item = Located<Token>>,
    scope: Scope,
    p: &mut Program,
) -> ParseResult<Block> {
    let mut tokens = tokens.into_iter().peekable();
    let mut program = vec![];

    while let Some(token) = tokens.next() {
        program.push(next_construct(token, &mut tokens, scope, p)?);
    }

    Ok(program)
}

pub fn ast(
    tokens: impl IntoIterator<Item = Located<Token>>,
    p: impl AsRef<Path>,
) -> ParseResult<Program> {
    let path = p.as_ref();
    let mut buff = path.to_path_buf();

    remove_file(&mut buff);

    top_level(tokens, buff.as_path())
}
