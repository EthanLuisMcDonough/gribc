mod analysis;
mod node;
mod opexpr;

pub use self::analysis::*;
pub use self::node::*;
use self::opexpr::{OpExpr, OpExprManager};
use lex::{Grouper, Keyword, Token};
use location::{Located, Location};
use operators::{op_precedence, Assignment, Binary, Precedence};
use std::convert::TryInto;
use util::next_if;

type ParseResult<T> = Result<T, ParseError>;
type Block = Vec<Node>;

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

// Assumes an even groupers
fn expression_list(tokens: Vec<Located<Token>>) -> ParseResult<Vec<Expression>> {
    let mut level = 0usize;
    let mut index = 0usize;
    let mut indices = vec![];

    for token in tokens.iter() {
        match token.data {
            Token::OpenGroup(_) => level += 1,
            Token::CloseGroup(_) => {
                level = level
                    .checked_sub(1)
                    .ok_or(ParseError::UnexpectedToken(token.clone()))?;
            }
            Token::Comma if level == 0 => {
                indices.push((std::mem::replace(&mut index, 0usize), token.clone()));
                continue;
            }
            _ => {}
        }
        index += 1;
    }

    let mut tokens = tokens.into_iter();
    let mut expressions = vec![];

    for (i, token) in indices.into_iter() {
        expressions.push(
            parse_expr(
                tokens
                    .by_ref()
                    .enumerate()
                    .take_while(|(index, _)| *index != i)
                    .map(|(_, val)| val)
                    .collect::<Vec<_>>(),
            )
            .map_err(|e| e.neof_or(ParseError::UnexpectedToken(token)))?,
        );
    }

    let remaining = tokens.collect::<Vec<_>>();

    if !remaining.is_empty() {
        expressions.push(parse_expr(remaining)?);
    }

    Ok(expressions)
}

fn parse_prop(tokens: impl IntoIterator<Item = Located<Token>>) -> ParseResult<AutoProp> {
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

fn parse_hash(tokens: impl IntoIterator<Item = Located<Token>>) -> ParseResult<Hash> {
    let mut tokens = tokens.into_iter().peekable();
    let mut map = Hash::new();
    while tokens.peek().is_some() {
        let key = next_guard!({ tokens.next() } {
            Token::Identifier(s) | Token::String(s) => s,
            Token::Number(n) => n.to_string()
        });
        let value = next_guard!({ tokens.next() } {
            Token::Arrow => match zero_level_preserve(&mut tokens, |t| *t == Token::Comma)? {
                Ok((tokens, _)) | Err(tokens) => parse_expr(tokens).map(ObjectValue::Expression)
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

fn lam_body(body: Vec<Located<Token>>) -> ParseResult<LambdaBody> {
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
            parse_expr(body)
                .map(Box::from)
                .map(LambdaBody::ImplicitReturn)
        }
    } else {
        ast_level(body, true, false).map(LambdaBody::Block)
    }
}

fn parse_expr(tokens: impl IntoIterator<Item = Located<Token>>) -> ParseResult<Expression> {
    let mut tokens = tokens.into_iter().peekable();
    let mut op_expr = OpExprManager::new();

    fn list_callback(
        (v, last): (Vec<Located<Token>>, Located<Token>),
    ) -> Result<Vec<Expression>, ParseError> {
        expression_list(v).map_err(|e| e.neof_or(ParseError::UnexpectedToken(last)))
    }

    fn expr_callback(
        (v, last): (Vec<Located<Token>>, Located<Token>),
    ) -> Result<Expression, ParseError> {
        parse_expr(v).map_err(|e| e.neof_or(ParseError::UnexpectedToken(last)))
    }

    while let Some(token) = tokens.next() {
        let mut expr = None;

        let data = token.data.clone();
        let start = token.start.clone();
        match data {
            Token::BinaryOp(binary) => op_expr
                .push(binary.clone())
                .map_err(|_| ParseError::UnexpectedToken(token.clone()))?,
            Token::UnaryOp(unary) => op_expr
                .push(unary.clone())
                .map_err(|_| ParseError::UnexpectedToken(token.clone()))?,
            Token::AssignOp(assign) => op_expr
                .push(assign.clone())
                .map_err(|_| ParseError::UnexpectedToken(token.clone()))?,
            Token::OpenGroup(Grouper::Bracket) => {
                expr = Expression::ArrayCreation(
                    take_until(&mut tokens, Grouper::Bracket).and_then(list_callback)?,
                )
                .into();
            }
            Token::OpenGroup(Grouper::Parentheses) => {
                expr = take_until(&mut tokens, Grouper::Parentheses)
                    .and_then(expr_callback)?
                    .into();
            }
            Token::Keyword(Keyword::Lam) => {
                let params = parse_params(&mut tokens)?;
                let (body, _) = take_until(&mut tokens, Grouper::Brace)?;

                expr = Expression::Lambda {
                    param_list: params,
                    body: lam_body(body)?,
                }
                .into();
            }
            Token::Hash | Token::MutableHash => {
                expr = next_guard!({ tokens.next() } { Token::OpenGroup(Grouper::Brace) => {
                    let (body, _) = take_until(&mut tokens, Grouper::Brace)?;
                    let hash = parse_hash(body)?;
                    if data == Token::Hash {
                        Expression::Hash(hash)
                    } else {
                        Expression::MutableHash(hash)
                    }
                } })
                .into();
            }
            Token::Keyword(Keyword::Nil) => expr = Expression::Nil.into(),
            Token::Bool(b) => expr = Expression::Bool(b).into(),
            Token::String(s) => expr = Expression::String(s).into(),
            Token::Number(n) => expr = Expression::Number(n).into(),
            Token::Identifier(data) => {
                expr = Expression::Identifier(Located {
                    data,
                    start: token.start.clone(),
                    end: token.end.clone(),
                })
                .into()
            }
            _ => return Err(ParseError::UnexpectedToken(token)),
        };

        if let Some(mut expression) = expr {
            while let Some(token) = next_if(tokens.by_ref(), |Located { data, .. }| !data.is_op()) {
                expression = match token.data {
                    Token::OpenGroup(Grouper::Parentheses) => Expression::FunctionCall {
                        function: expression.into(),
                        args: take_until(&mut tokens, Grouper::Parentheses)
                            .and_then(list_callback)?,
                    },
                    Token::OpenGroup(Grouper::Bracket) => Expression::IndexAccess {
                        item: expression.into(),
                        index: take_until(&mut tokens, Grouper::Bracket)
                            .and_then(expr_callback)?
                            .into(),
                    },
                    Token::Period => next_guard!({ tokens.next() } {
                        Token::Keyword(k) => Expression::PropertyAccess {
                            item: expression.into(),
                            property: k.str().into(),
                        },
                        Token::Identifier(property) => Expression::PropertyAccess {
                            item: expression.into(),
                            property,
                        }
                    }),
                    _ => return Err(ParseError::UnexpectedToken(token)),
                }
            }

            op_expr
                .push((expression, start))
                .map_err(|_| ParseError::UnexpectedToken(token))?;
        }
    }

    let mut op_expr: Vec<_> = op_expr.into();

    for index in op_expr
        .iter()
        .enumerate()
        .flat_map(|(i, op_e)| Some(i).filter(|_| op_e.is_unary()))
        .rev()
        .collect::<Vec<_>>()
    {
        if let OpExpr::Unary(unary) = op_expr.remove(index) {
            if let Some(OpExpr::Expr(ref mut expr, _)) = op_expr.get_mut(index) {
                *expr = Expression::Unary {
                    op: unary,
                    expr: std::mem::replace(expr, Expression::Nil).into(),
                };
            } else {
                return Err(ParseError::UnexpectedEOF);
            }
        }
    }

    for precedence in Precedence::ITEMS.iter().rev() {
        for i in op_expr
            .iter()
            .enumerate()
            .flat_map(|(i, op)| match op {
                OpExpr::Binary(b) if *precedence == op_precedence(b) => Some(i),
                _ => None,
            })
            .enumerate()
            .map(|(i, index)| index - i * 2)
            .collect::<Vec<_>>()
        {
            if let (OpExpr::Expr(one, _), OpExpr::Binary(op)) =
                (op_expr.remove(i - 1), op_expr.remove(i - 1))
            {
                if let Some(OpExpr::Expr(ref mut expr, _)) = op_expr.get_mut(i - 1) {
                    *expr = Expression::Binary {
                        op,
                        left: one.into(),
                        right: std::mem::replace(expr, Expression::Nil).into(),
                    }
                } else {
                    return Err(ParseError::UnexpectedEOF);
                }
            }
        }
    }

    for i in op_expr
        .iter()
        .enumerate()
        .flat_map(|(i, v)| Some(i).filter(|_| v.is_assign()))
        .enumerate()
        .map(|(i, index)| index - i * 2)
        .collect::<Vec<_>>()
    {
        if let (OpExpr::Expr(one, start), OpExpr::Assign(op)) =
            (op_expr.remove(i - 1), op_expr.remove(i - 1))
        {
            if let Some(OpExpr::Expr(ref mut expr, _)) = op_expr.get_mut(i - 1) {
                *expr = Expression::Assignment {
                    op,
                    left: one
                        .try_into()
                        .map_err(|_| ParseError::IllegalLeftExpression { start })?,
                    right: std::mem::replace(expr, Expression::Nil).into(),
                }
            } else {
                return Err(ParseError::UnexpectedEOF);
            }
        }
    }

    op_expr
        .pop()
        .filter(|_| op_expr.is_empty())
        .and_then(|expr| {
            if let OpExpr::Expr(e, _) = expr {
                Some(e)
            } else {
                None
            }
        })
        .ok_or(ParseError::UnexpectedEOF)
}

fn zero_level<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    predicate: impl Fn(&Token) -> bool,
) -> ParseResult<(Vec<Located<Token>>, Located<Token>)> {
    zero_level_preserve(tokens, predicate)?.map_err(|_| ParseError::UnexpectedEOF)
}

fn zero_level_preserve<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    predicate: impl Fn(&Token) -> bool,
) -> ParseResult<Result<(Vec<Located<Token>>, Located<Token>), Vec<Located<Token>>>> {
    let mut stack = vec![];
    let mut selected = vec![];

    for token in tokens {
        if stack.is_empty() && predicate(&token.data) {
            return Ok(Ok((selected, token)));
        }

        if let Token::OpenGroup(g) = &token.data {
            stack.push(g.clone());
        } else if let Token::CloseGroup(g) = &token.data {
            stack
                .pop()
                .filter(|lg| lg == g)
                .ok_or(ParseError::UnexpectedToken(token.clone()))?;
        }

        selected.push(token);
    }

    Ok(Err(selected))
}

fn take_until<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    with: Grouper,
) -> ParseResult<(Vec<Located<Token>>, Located<Token>)> {
    let data = Token::CloseGroup(with);
    zero_level(tokens, |t| *t == data)
}

fn parse_if_block<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    in_fn: bool,
    in_loop: bool,
) -> ParseResult<ConditionBodyPair> {
    Ok(ConditionBodyPair {
        condition: zero_level(tokens, |t| *t == Token::OpenGroup(Grouper::Brace))
            .and_then(|(v, _)| parse_expr(v))?,
        block: take_until(tokens, Grouper::Brace)
            .and_then(|(v, _)| ast_level(v, in_fn, in_loop))?,
    })
}

fn parse_decl<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    mutable: bool,
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
                    parse_expr(v)?
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

fn parse_params<T: Iterator<Item = Located<Token>>>(tokens: &mut T) -> ParseResult<Parameters> {
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

fn ast_level(
    tokens: impl IntoIterator<Item = Located<Token>>,
    in_fn: bool,
    in_loop: bool,
) -> ParseResult<Block> {
    let mut tokens = tokens.into_iter().peekable();
    let mut program = vec![];

    while let Some(token) = tokens.next() {
        program.push(match token.data {
            Token::OpenGroup(Grouper::Bracket) => {
                Node::Block(take_until(&mut tokens, Grouper::Brace).and_then(|(v, _)| ast_level(v, in_fn, in_loop))?)
            }
            Token::Keyword(Keyword::While) => parse_if_block(&mut tokens, in_fn, true).map(Node::While)?,
            Token::Keyword(Keyword::If) => {
                let if_block = parse_if_block(&mut tokens, in_fn, in_loop)?;
                let mut elseifs = vec![];
                let mut else_block = None;

                while let Some(_) = next_if(tokens.by_ref(), |Located { data: t, .. }| {
                    *t == Token::Keyword(Keyword::Else) && else_block.is_none()
                }) {
                    next_guard!({ tokens.next() } {
                        Token::OpenGroup(Grouper::Brace) => else_block = take_until(&mut tokens, Grouper::Brace)
                            .and_then(|(v, _)| ast_level(v, in_fn, in_loop))?.into(),
                        Token::Keyword(Keyword::If) => elseifs.push(parse_if_block(&mut tokens, in_fn, in_loop)?)
                    });
                }

                Node::LogicChain {
                    if_block,
                    elseifs,
                    else_block,
                }
            }
            Token::Keyword(Keyword::Break) if !in_loop => return Err(ParseError::IllegalBreak(token.start)),
            Token::Keyword(Keyword::Continue) if !in_loop => return Err(ParseError::IllegalContinue(token.start)),
            Token::Keyword(Keyword::Break) => next_guard!({ tokens.next() } {
                Token::Semicolon => Node::Break
            }),
            Token::Keyword(Keyword::Continue) => next_guard!({ tokens.next() } {
                Token::Semicolon => Node::Continue
            }),
            Token::Keyword(Keyword::Return) if !in_fn => return Err(ParseError::IllegalReturn(token.start)),
            Token::Keyword(Keyword::Return) => {
                let (tokens, _) = zero_level(&mut tokens, |t| *t == Token::Semicolon)?;
                Node::Return(if tokens.is_empty() { Expression::Nil } else { parse_expr(tokens)? })
            },
            Token::Keyword(Keyword::Decl) => Node::Declaration(parse_decl(&mut tokens, true)?),
            Token::Keyword(Keyword::Im) => Node::Declaration(parse_decl(&mut tokens, false)?),
            Token::Keyword(Keyword::Proc) => {
                let name = next_guard!({ tokens.next() } (start, end) {
                    Token::Identifier(i) => Located { data: i, start, end }
                });
                let param_list = parse_params(&mut tokens)?;
                Node::Procedure {
                    identifier: name,
                    param_list,
                    body: take_until(&mut tokens, Grouper::Brace).and_then(|(v, _)| ast_level(v, true, false))?,
                }
            },
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

                let body = take_until(&mut tokens, Grouper::Brace).and_then(|(v, _)| ast_level(v, in_fn, true))?;

                Node::For {
                    declaration,
                    condition,
                    increment,
                    body,
                }
            }
            _ => {
                let loc = token.start.clone();
                let (mut tokens, _) = zero_level(&mut tokens, |t| *t == Token::Semicolon)?;
                tokens.insert(0, token);
                let expr = parse_expr(tokens)?;
                if !expr.is_statement() {
                    return Err(ParseError::IllegalExpression(loc));
                }
                Node::Expression(expr)
            }
        });
    }

    Ok(program)
}

pub fn ast(tokens: impl IntoIterator<Item = Located<Token>>) -> ParseResult<Block> {
    ast_level(tokens, false, false)
}
