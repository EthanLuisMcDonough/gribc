use crate::keyword_map;
use operators::{Assignment, Binary, Unary};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Grouper {
    Bracket,
    Parentheses,
    Brace,
}

keyword_map!(Keyword {
    Proc -> "proc",
    Lam -> "lam",
    Return -> "return",
    Decl -> "decl",
    Im -> "im",
    While -> "while",
    For -> "for",
    Nil -> "nil",
    If -> "if",
    Else -> "else",
    Break -> "break",
    Continue -> "continue",
    Get -> "get",
    Set -> "set",
    Import -> "import",
    Public -> "public",
    From -> "from",
});

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Token {
    String(String),
    Number(f64),
    Identifier(String),
    BinaryOp(Binary),
    UnaryOp(Unary),
    AssignOp(Assignment),
    Arrow,

    Period,
    Spread,

    MutableHash,
    Hash,

    // Keywords
    Keyword(Keyword),

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
