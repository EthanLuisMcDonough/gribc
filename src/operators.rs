macro_rules! list_enum {
    (count [ $item:ident ]) => {
        1
    };
    (count [ $i:ident, $( $item:ident ),+ ]) => {
        1 + list_enum!(count [ $( $item ),* ])
    };
    ($name:ident { $( $item:ident ),* }) => {
        #[derive(Clone, Debug, PartialEq)]
        pub enum $name {
            $( $item, )*
        }

        impl $name {
            pub const ITEMS: [$name; list_enum!(count [ $( $item ),* ])] = [ $( $name::$item ),* ];
        }
    };
}

list_enum!(Precedence {
    LogOr,    // Logical OR
    LogAnd,   // Logical AND
    Equality, // Equality operators
    RelLog,   // Binary logical operators
    AddSub,   // Addition and subtraction
    MultDiv   // Division and multiplication
});

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Assignment {
    Assign,
    AssignPlus,
    AssignMinus,
    AssignMult,
    AssignDiv,
    AssignMod,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Binary {
    Plus,
    Minus,
    Mult,
    Div,
    Mod,
    GreaterThan,
    LessThan,
    GreaterEq,
    LessEq,
    Equal,
    NotEqual,
    LogicalAnd,
    LogicalOr,
}

impl Binary {
    pub fn is_lazy(&self) -> bool {
        match self {
            Binary::LogicalAnd | Binary::LogicalOr => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Unary {
    Negation,
    LogicalNegation,
}

pub fn op_precedence(op: &Binary) -> Precedence {
    use self::Binary::*;
    match op {
        Mult | Div | Mod => Precedence::MultDiv,
        Plus | Minus => Precedence::AddSub,
        GreaterEq | GreaterThan | LessEq | LessThan | Equal | NotEqual => Precedence::RelLog,
        LogicalAnd => Precedence::LogAnd,
        LogicalOr => Precedence::LogOr,
    }
}
