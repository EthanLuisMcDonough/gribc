mod scope;
mod util;
mod constructs;
mod expression;

use ast::node::*;
use self::scope::Scope;
use self::util::*;
use self::expression::parse_expr;
use self::constructs::*;
use lex::{tokens::*, LexError};
use location::{Located, Location};
use util::{next_if, remove_file};
use std::{convert::AsRef, path::Path};

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
    IllegalLeftExpression { start: Location },
    IllegalExpression(Location),
    DuplicateParam(Located<String>),
    ParamAfterSpread(Located<String>),
    IllegallyScopedImport(Location),
    FunctionNotAtTopLevel(Location),
    ModuleError(ModuleError),
    MisplacedImport(Location),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ModuleErrorBody {
    LexError(LexError),
    ParseError(Box<ParseError>),
    PathNotFound,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ModuleError {
    pub path: Located<String>,
    pub data: ModuleErrorBody
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

fn ast_level<'a>(
    tokens: impl IntoIterator<Item = Located<Token>>,
    scope: Scope<'a>,
) -> ParseResult<Block> {

    let mut tokens = tokens.into_iter().peekable();
    let mut program = vec![];

    let mut can_import = scope.is_top;

    while let Some(token) = tokens.next() {
        let mut was_import = false;

        program.push(match token.data {
            Token::OpenGroup(Grouper::Bracket) => {
                Node::Block(take_until(&mut tokens, Grouper::Brace).and_then(|(v, _)| ast_level(v, scope.next_level()))?)
            }
            Token::Keyword(Keyword::While) => parse_if_block(&mut tokens, scope.next_level().with_loop(true)).map(Node::While)?,
            Token::Keyword(Keyword::If) => {
                let if_block = parse_if_block(&mut tokens, scope.next_level())?;
                let mut elseifs = vec![];
                let mut else_block = None;

                while let Some(_) = next_if(tokens.by_ref(), |Located { data: t, .. }| {
                    *t == Token::Keyword(Keyword::Else) && else_block.is_none()
                }) {
                    next_guard!({ tokens.next() } {
                        Token::OpenGroup(Grouper::Brace) => else_block = take_until(&mut tokens, Grouper::Brace)
                            .and_then(|(v, _)| ast_level(v, scope.next_level()))?.into(),
                        Token::Keyword(Keyword::If) => elseifs.push(parse_if_block(&mut tokens, scope.next_level())?)
                    });
                }

                Node::LogicChain {
                    if_block,
                    elseifs,
                    else_block,
                }
            }
            Token::Keyword(Keyword::Break) if !scope.in_loop => return Err(ParseError::IllegalBreak(token.start)),
            Token::Keyword(Keyword::Continue) if !scope.in_loop => return Err(ParseError::IllegalContinue(token.start)),
            Token::Keyword(Keyword::Break) => next_guard!({ tokens.next() } {
                Token::Semicolon => Node::Break
            }),
            Token::Keyword(Keyword::Continue) => next_guard!({ tokens.next() } {
                Token::Semicolon => Node::Continue
            }),
            Token::Keyword(Keyword::Return) if !scope.in_proc => return Err(ParseError::IllegalReturn(token.start)),
            Token::Keyword(Keyword::Return) => {
                let (tokens, _) = zero_level(&mut tokens, |t| *t == Token::Semicolon)?;
                Node::Return(if tokens.is_empty() { Expression::Nil } else { parse_expr(tokens)? })
            },
            Token::Keyword(Keyword::Decl) => Node::Declaration(parse_decl(&mut tokens, true)?),
            Token::Keyword(Keyword::Im) => Node::Declaration(parse_decl(&mut tokens, false)?),
            Token::Keyword(Keyword::Public) if scope.is_top => next_guard!({ tokens.next() } {
                Token::Keyword(Keyword::Proc) => Node::Procedure(parse_proc(&mut tokens, true)?)
            }),
            Token::Keyword(Keyword::Proc) if scope.is_top => Node::Procedure(parse_proc(&mut tokens, false)?),
            Token::Keyword(Keyword::Public) | Token::Keyword(Keyword::Proc) => 
                return Err(ParseError::FunctionNotAtTopLevel(token.start.clone())),
            Token::Keyword(Keyword::For) => {
                let declaration = next_guard!({ tokens.next() } {
                    Token::Keyword(Keyword::Decl) => parse_decl(&mut tokens, true)?.into(),
                    Token::Keyword(Keyword::Im) => parse_decl(&mut tokens, false)?.into(),
                    Token::Semicolon => None
                });

                let (t, _) = zero_level(&mut tokens, |d| *d == Token::Semicolon)?;
                let condition = if t.is_empty() { None } else { parse_expr(t)?.into() };

                let (t, _) = zero_level(&mut tokens, |d| *d == Token::OpenGroup(Grouper::Brace))?;
                let increment = if t.is_empty() { None } else { parse_expr(t)?.into() };

                let body = take_until(&mut tokens, Grouper::Brace)
                    .and_then(|(v, _)| ast_level(v, scope.next_level().with_loop(true)))?;

                Node::For {
                    declaration,
                    condition,
                    increment,
                    body,
                }
            }
            Token::Keyword(Keyword::Import) if scope.is_top && can_import => {
                was_import = true;
                Node::Import(parse_import(&mut tokens, scope)?)
            },
            Token::Keyword(Keyword::Import) => 
                return Err(ParseError::MisplacedImport(token.start.clone())),
            Token::Semicolon => return Err(ParseError::UnexpectedToken(Located {
                data: Token::Semicolon,
                start: token.start.clone(),
                end: token.end.clone(),
            })),
            _ => {
                let loc = token.start.clone();
                let (mut tokens, semi) = zero_level(&mut tokens, |t| *t == Token::Semicolon)?;
                tokens.insert(0, token);

                let expr = parse_expr(tokens).map_err(|e| 
                    e.neof_or(ParseError::UnexpectedToken(semi)))?;

                if !expr.is_statement() {
                    return Err(ParseError::IllegalExpression(loc));
                }

                Node::Expression(expr)
            }
        });

        can_import &= was_import;
    }

    Ok(program)
}

pub fn ast(tokens: impl IntoIterator<Item = Located<Token>>, p: impl AsRef<Path>) -> ParseResult<Block> {
    let path = p.as_ref();
    let mut buff = path.to_path_buf();

    remove_file(&mut buff);
    
    let data = Scope::new(buff.as_path());
    ast_level(tokens, data)
}
