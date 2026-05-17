use serde::Serialize;

use super::*;

#[derive(Debug, Clone, Serialize)]
pub enum BinaryOp {
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    And,
    Or,
    ShiftLeft,
    ShiftRight,
    RangeInclusive,
    RangeExclusive,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Debug, Clone, Serialize)]
pub enum Expression {
    Literal(Literal),
    Path(ExprPath), // Updated from Path to support paths containing turbofish segments
    Fix {
        initial: Box<Expression>,
        prefixes: Vec<Prefix>,
        postfixes: Vec<Postfix>,
    },
    Binary {
        lhs: Box<Expression>,
        op: BinaryOp,
        rhs: Box<Expression>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub enum Literal {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Tuple(Vec<Expression>),
    Array(Vec<Expression>),
    ArrayRepeat(Box<Expression>, Box<Expression>),
}

#[derive(Debug, Clone, Serialize)]
pub enum Pattern {
    NamedTuple(Path, Vec<Identifier>),
    Struct(Path, Vec<Identifier>),
    Tuple(Vec<Identifier>),
    Literal(Literal),
    Path(Path),
    Id(Identifier),
}

#[derive(Debug, Clone, Serialize)]
pub enum Postfix {
    FieldAccess(Identifier, Option<Generics>), // Updated to support method turbofish chains
    Call(Vec<Expression>),
    MacroCall(String),
    StructCall(Vec<(Identifier, Expression)>),
    Index(Expression),
    As(TypeExpr),
    RangeInclusive,
    RangeExclusive,
}

#[derive(Debug, Clone, Serialize)]
pub enum Prefix {
    Ref,
    RefMut,
    Deref,
    New,
    Not,
    Neg,
    RangeInclusive,
    RangeExclusive,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExprPathSegment {
    pub ident: Identifier,
    pub generics: Option<Generics>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExprPath {
    pub segments: Vec<ExprPathSegment>,
}
