
use lex::tokens::{Grouper, Token};
use location::Located;
use ast::{ParseError, ParseResult};

pub fn zero_level<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    predicate: impl Fn(&Token) -> bool,
) -> ParseResult<(Vec<Located<Token>>, Located<Token>)> {
    zero_level_preserve(tokens, predicate)?.map_err(|_| ParseError::UnexpectedEOF)
}

pub fn zero_level_preserve<T: Iterator<Item = Located<Token>>>(
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

pub fn take_until<T: Iterator<Item = Located<Token>>>(
    tokens: &mut T,
    with: Grouper,
) -> ParseResult<(Vec<Located<Token>>, Located<Token>)> {
    let data = Token::CloseGroup(with);
    zero_level(tokens, |t| *t == data)
}