mod constructs;
pub(in ast::parsing) mod data_store;
mod expression;
mod util;

use self::constructs::*;
use self::data_store::Store;
use self::expression::parse_expr;
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
        }
    };
    ({ $next:expr } { $( $token:tt )* }) => {
        next_guard!({ $next } (_s, _e) { $( $token )* })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ParseError {
    UnexpectedEOF,
    UnexpectedToken(Located<Token>),

    IllegallyScopedImport(Location),
    IllegalExpression(Location),
    IllegalLeftExpression(Location),

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
    store: &mut Store,
) -> ParseResult<Node> {
    Ok(match token.data {
        Token::OpenGroup(Grouper::Bracket) => {
            Node::Block(take_until(tokens, Grouper::Brace).and_then(|(v, _)| ast_level(v, store))?)
        }

        Token::Keyword(Keyword::While) => parse_if_block(tokens, store).map(Node::While)?,
        Token::Keyword(Keyword::If) => {
            let if_block = parse_if_block(tokens, store)?;
            let mut elseifs = vec![];
            let mut else_block = None;

            while let Some(_) = next_if(tokens, |Located { data: t, .. }| {
                *t == Token::Keyword(Keyword::Else) && else_block.is_none()
            }) {
                next_guard!({ tokens.next() } {
                    Token::OpenGroup(Grouper::Brace) => else_block = take_until(tokens, Grouper::Brace)
                        .and_then(|(v, _)| ast_level(v, store))?.into(),
                    Token::Keyword(Keyword::If) => elseifs.push(parse_if_block(tokens, store)?)
                });
            }

            Node::LogicChain {
                if_block,
                elseifs,
                else_block,
            }
        }

        Token::Keyword(Keyword::Break) => next_guard!({ tokens.next() }
            { Token::Semicolon => Node::ControlFlow(FlowBreak::new_break(token.start, token.end)) }),
        Token::Keyword(Keyword::Continue) => next_guard!({ tokens.next() }
            { Token::Semicolon => Node::ControlFlow(FlowBreak::new_continue(token.start, token.end)) }),
        Token::Keyword(Keyword::Return) => {
            let (tokens, _) = zero_level(tokens, |t| *t == Token::Semicolon)?;
            let expr = if tokens.is_empty() {
                Expression::Nil
            } else {
                parse_expr(tokens, store)?
            };
            Node::ControlFlow(FlowBreak::new_return(expr, token.start, token.end))
        }

        Token::Keyword(Keyword::Decl) => Node::Declaration(parse_decl(tokens, true, store)?),
        Token::Keyword(Keyword::Im) => Node::Declaration(parse_decl(tokens, false, store)?),

        Token::Keyword(Keyword::Proc) | Token::Keyword(Keyword::Public) => {
            return Err(ParseError::FunctionNotAtTopLevel(token.start.clone()))
        }
        Token::Keyword(Keyword::Import) => {
            return Err(ParseError::MisplacedImport(token.start.clone()))
        }

        Token::Keyword(Keyword::For) => {
            let declaration = next_guard!({ tokens.next() } {
                Token::Keyword(Keyword::Decl) => parse_decl(tokens, true, store)?.into(),
                Token::Keyword(Keyword::Im) => parse_decl(tokens, false, store)?.into(),
                Token::Semicolon => None
            });

            let (t, _) = zero_level(tokens, |d| *d == Token::Semicolon)?;
            let condition = if t.is_empty() {
                None
            } else {
                parse_expr(t, store)?.into()
            };

            let (t, _) = zero_level(tokens, |d| *d == Token::OpenGroup(Grouper::Brace))?;
            let increment = if t.is_empty() {
                None
            } else {
                parse_expr(t, store)?.into()
            };

            let body = take_until(tokens, Grouper::Brace).and_then(|(v, _)| ast_level(v, store))?;

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

            let expr = parse_expr(tokens, store)
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

    let mut store = Store::new();
    let mut tokens = tokens.into_iter().peekable();
    let mut body = Block::default();

    while next_if(&mut tokens, is_import).is_some() {
        let import = parse_import(&mut tokens, path, &mut store)?;
        store.add_import(import);
    }

    while let Some(token) = tokens.next() {
        match token.data {
            Token::Keyword(Keyword::Proc) => {
                let proc = parse_proc(&mut tokens, false, &mut store)?;
                store.add_fn(proc);
            }
            Token::Keyword(Keyword::Public) => next_guard!({ tokens.next() } {
                Token::Keyword(Keyword::Proc) => {
                    let proc = parse_proc(&mut tokens, true, &mut store)?;
                    store.add_fn(proc);
                }
            }),
            _ => {
                let construct = next_construct(token.clone(), &mut tokens, &mut store)?;
                body.push(construct);
            }
        };
    }

    let mut program = Program::from(store);
    program.body = body;

    Ok(program)
}

fn ast_level(
    tokens: impl IntoIterator<Item = Located<Token>>,
    store: &mut Store,
) -> ParseResult<Block> {
    let mut tokens = tokens.into_iter().peekable();
    let mut program = Block::new();

    while let Some(token) = tokens.next() {
        program.push(next_construct(token, &mut tokens, store)?);
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
