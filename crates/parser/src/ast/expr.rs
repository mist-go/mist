use serde::Serialize;

use super::*;

#[derive(Debug, Clone, Serialize)]
pub enum Expression {
    Literal(Literal),
    Path(ExprPath),
    Statement(Box<Statement>),
    Fix {
        initial: Box<Expression>,
        prefixes: Vec<Prefix>,
        postfixes: Vec<Postfix>,
    },
    Binary {
        lhs: Box<Expression>,
        op: String,
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
    FieldAccess(Identifier, Option<Generics>),
    Call(Vec<Expression>),
    MacroCall(String),
    StructCall(Vec<(Identifier, Expression)>),
    Assign(String, Box<Expression>),
    Index(Expression),
    As(TypeExpr),
    Increment,
    Decrement,
    Try,
}

#[derive(Debug, Clone, Serialize)]
pub enum Prefix {
    Ref,
    RefMut,
    Deref,
    New(Option<Generics>),
    Not,
    Neg,
    Closure(Option<TypeExpr>, Vec<VarDecl>),
}

#[derive(Debug, Clone, Serialize)]
pub struct ExprPathSegment {
    pub ident: Identifier,
    pub generics: Option<Generics>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExprPath(pub Vec<ExprPathSegment>);

#[derive(Debug, Clone, Serialize)]
pub struct Generics(pub Vec<Generic>);

#[derive(Debug, Clone, Serialize)]
pub enum Generic {
    Lifetime(Identifier),
    Type(TypeExpr),
}

impl Expression {
    pub fn is_block(&self) -> bool {
        if let Expression::Statement(stmt) = self {
            stmt.is_block()
        } else {
            false
        }
    }
}
