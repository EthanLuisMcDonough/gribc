use location::{Located, Location};
use operators::{Assignment, Binary, Unary};
use util::next_if;

type LexResult<A> = Result<A, LexError>;

#[derive(Clone, Debug, PartialEq)]
pub enum LexErrorData {
    UnexpectedEOF,
    UnexpectedChar(char),
    InvalidNumber(String),
}

impl LexErrorData {
    fn with_loc(self, loc: Location) -> LexError {
        LexError { loc, data: self }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexError {
    pub loc: Location,
    pub data: LexErrorData,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for LexError {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Grouper {
    Bracket,
    Parentheses,
    Brace,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Token {
    String(String),
    Number(f64),
    Identifier(String),
    BinaryOp(Binary),
    UnaryOp(Unary),
    AssignOp(Assignment),

    // Keywords
    Proc,
    Lam,
    Return,
    Decl,
    Im,
    While,
    For,
    Nil,
    If,
    Else,
    Args,
    Break,
    Continue,
    Period,
    Spread,
    MutableHash,
    Hash,
    Arrow,
    Get,
    Set,

    // Booleans
    Bool(bool),

    // Delimiters
    Comma,
    Semicolon,
    Pipe,

    // Grouping tokens
    OpenGroup(Grouper),
    CloseGroup(Grouper),
}

impl Token {
    pub fn is_op(&self) -> bool {
        match self {
            Token::UnaryOp(_) | Token::BinaryOp(_) | Token::AssignOp(_) => true,
            _ => false,
        }
    }

    pub fn ident(&self) -> bool {
        if let Token::Identifier(_) = self {
            true
        } else {
            false
        }
    }
}

fn nchar_if<T: Iterator<Item = char>>(
    i: &mut std::iter::Peekable<T>,
    c: char,
    loc: &mut Location,
) -> Option<char> {
    next_if(i, |ch| *ch == c).map(|c| {
        loc.feed(c);
        c
    })
}

fn next_guard<T: Iterator<Item = char>>(
    i: &mut T,
    c: char,
    loc: &mut Location,
) -> Result<(), LexError> {
    match i.next() {
        Some(ch) => {
            if ch == c {
                loc.feed(ch);
                Ok(())
            } else {
                Err(LexErrorData::UnexpectedChar(ch).with_loc(loc.clone()))
            }
        }
        None => Err(LexErrorData::UnexpectedEOF.with_loc(loc.clone())),
    }
}

pub fn lex(s: &str) -> LexResult<Vec<Located<Token>>> {
    let mut chars = s.chars().peekable();
    let mut loc = Location::new();
    let mut tokens = vec![];

    while let Some(c) = chars.next() {
        let start = loc.clone();
        loc.feed(c);
        tokens.push(Located::with_loc(
            match c {
                '[' => Token::OpenGroup(Grouper::Bracket),
                ']' => Token::CloseGroup(Grouper::Bracket),
                '(' => Token::OpenGroup(Grouper::Parentheses),
                ')' => Token::CloseGroup(Grouper::Parentheses),
                '{' => Token::OpenGroup(Grouper::Brace),
                '}' => Token::CloseGroup(Grouper::Brace),
                ',' => Token::Comma,
                ';' => Token::Semicolon,
                '.' => {
                    if nchar_if(&mut chars, '.', &mut loc).is_some() {
                        next_guard(&mut chars, '.', &mut loc)?;
                        Token::Spread
                    } else {
                        Token::Period
                    }
                }
                '$' => Token::MutableHash,
                '#' => Token::Hash,
                '0'...'9' => {
                    let mut number = c.to_string();
                    let mut period = false;

                    while let Some(c) =
                        next_if(&mut chars, |&c| c.is_digit(10) || (!period && c == '.'))
                    {
                        loc.feed(c);
                        number.push(c);
                        period = period || c == '.';
                    }

                    Token::Number(
                        number.parse().map_err(|_| {
                            LexErrorData::InvalidNumber(number).with_loc(loc.clone())
                        })?,
                    )
                }
                '|' => {
                    if nchar_if(&mut chars, '|', &mut loc).is_some() {
                        Token::BinaryOp(Binary::LogicalOr)
                    } else {
                        Token::Pipe
                    }
                }
                '&' => {
                    next_guard(&mut chars, '&', &mut loc)?;
                    Token::BinaryOp(Binary::LogicalAnd)
                }
                '+' | '-' | '*' | '/' | '<' | '>' | '%' => {
                    let eq = nchar_if(&mut chars, '=', &mut loc).is_some();

                    if next_if(&mut chars, |&nc| !eq && c == '-' && nc == '>').is_some() {
                        loc.feed('>');
                        Token::Arrow
                    } else if eq {
                        match c {
                            '+' if eq => Token::AssignOp(Assignment::AssignPlus),
                            '-' if eq => Token::AssignOp(Assignment::AssignMinus),
                            '*' if eq => Token::AssignOp(Assignment::AssignMult),
                            '/' if eq => Token::AssignOp(Assignment::AssignDiv),
                            '%' if eq => Token::AssignOp(Assignment::AssignMod),
                            '>' if eq => Token::BinaryOp(Binary::GreaterEq),
                            '<' if eq => Token::BinaryOp(Binary::LessEq),
                            _ => return Err(LexErrorData::UnexpectedChar(c).with_loc(loc.clone())),
                        }
                    } else {
                        Token::BinaryOp(match c {
                            '+' => Binary::Plus,
                            '-' => Binary::Minus,
                            '*' => Binary::Mult,
                            '/' => Binary::Div,
                            '%' => Binary::Mod,
                            '>' => Binary::GreaterThan,
                            '<' => Binary::LessThan,
                            _ => return Err(LexErrorData::UnexpectedChar(c).with_loc(loc.clone())),
                        })
                    }
                }
                '!' => nchar_if(&mut chars, '=', &mut loc)
                    .map(|_| Token::BinaryOp(Binary::NotEqual))
                    .unwrap_or(Token::UnaryOp(Unary::LogicalNegation)),
                '~' => Token::UnaryOp(Unary::Negation),
                '=' => nchar_if(&mut chars, '=', &mut loc)
                    .map(|_| Token::BinaryOp(Binary::Equal))
                    .unwrap_or(Token::AssignOp(Assignment::Assign)),
                '"' => {
                    let mut string = String::new();
                    let mut complete = false;

                    while let Some(c) = chars.next().filter(|&c| {
                        loc.feed(c);
                        complete = c == '"';
                        !complete
                    }) {
                        string.push(if c == '\\' {
                            let ch = chars
                                .next()
                                .ok_or(LexErrorData::UnexpectedEOF.with_loc(loc.clone()))?;
                            loc.feed(ch);
                            ch
                        } else {
                            c
                        });
                    }

                    if !complete {
                        return Err(LexErrorData::UnexpectedEOF.with_loc(loc.clone()));
                    }

                    Token::String(string)
                }
                'A'...'Z' | 'a'...'z' | '_' => {
                    let mut ident = c.to_string();

                    while let Some(c) = next_if(&mut chars, |&c| valid_ident_char(c)) {
                        loc.feed(c);
                        ident.push(c);
                    }

                    match &ident[..] {
                        "proc" => Token::Proc,
                        "return" => Token::Return,
                        "decl" => Token::Decl,
                        "im" => Token::Im,
                        "while" => Token::While,
                        "for" => Token::For,
                        "nil" => Token::Nil,
                        "if" => Token::If,
                        "else" => Token::Else,
                        "true" | "false" => Token::Bool(ident == "true"),
                        "args" => Token::Args,
                        "NaN" => Token::Number(std::f64::NAN),
                        "Infinity" => Token::Number(std::f64::INFINITY),
                        "break" => Token::Break,
                        "continue" => Token::Continue,
                        "lam" => Token::Lam,
                        "typeof" => Token::UnaryOp(Unary::TypeOf),
                        "set" => Token::Set,
                        "get" => Token::Get,
                        _ => Token::Identifier(ident),
                    }
                }
                '@' => {
                    for _ in chars.by_ref().take_while(|&c| {
                        loc.feed(c);
                        c != '\n'
                    }) {}
                    continue;
                }
                _ if c.is_whitespace() => continue,
                _ => return Err(LexErrorData::UnexpectedChar(c).with_loc(loc.clone())),
            },
            start,
            loc.clone(),
        ));
    }

    Ok(tokens)
}

fn valid_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
