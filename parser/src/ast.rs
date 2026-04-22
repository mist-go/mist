use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ParamList(pub HashMap<String, (bool, TypeExpr)>);

#[derive(Debug, Clone, Serialize)]
pub struct Block(pub Vec<Statement>);

#[derive(Debug, Clone, Serialize)]
pub enum TypeExpr {
    Identifier(String),
}

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
}

#[derive(Debug, Clone, Serialize)]
pub enum TopLevel {
    Import(String),
    Package(String),
    StructDecl {
        export: bool,
        name: String,
        fields: ParamList,
    },
    FunctionDecl {
        export: bool,
        name: String,
        params: ParamList,
        return_type: Option<TypeExpr>,
        body: Block,
    },
}

#[derive(Debug, Clone, Serialize)]
pub enum Postfix {
    FieldAccess(String),
    Call(Vec<Expression>),
    Index(Expression),
    Binary(BinaryOp, Expression),
}

#[derive(Debug, Clone, Serialize)]
pub enum Statement {
    // expr;
    Expression(Expression),

    // { ... }
    Block(Block),

    // let/const/var x = ...
    VarDecl {
        kind: VarKind,
        name: String,
        init: Option<Expression>,
        type_: Option<TypeExpr>,
    },

    VarAssign {
        target: Expression,
        value: Expression,
    },

    // if (...) stmt else stmt
    If {
        condition: Expression,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },

    // while (...) stmt
    While {
        condition: Expression,
        body: Box<Statement>,
    },

    // for (...) stmt
    For {
        init: (VarKind, String, Option<Expression>),
        condition: Option<Expression>,
        update: Option<Box<Statement>>,
        body: Box<Statement>,
    },

    // return expr?;
    Return(Option<Expression>),

    Break,
    Continue,
}

#[derive(Debug, Clone, Serialize)]
pub enum Expression {
    Identifier(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    StringLiteral(String),
    Postfix {
        initial: Box<Expression>,
        postfixes: Vec<Postfix>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub enum VarKind {
    Let,
    Const,
    Var,
}
