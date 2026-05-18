use serde::Serialize;

use super::*;

#[derive(Debug, Clone, Serialize, Default)]
pub struct Block(pub Vec<Spanned<Statement>>);

#[derive(Debug, Clone, Serialize)]
pub enum Statement {
    Expression(Expression),
    Block(Block),

    VarDecl(VarDeclStmt),
    Assign {
        target: Expression,
        compound: String,
        value: Expression,
    },
    If {
        initial: StatementBranch,
        else_if: Vec<StatementBranch>,
        else_branch: Option<Box<Statement>>,
    },
    While(StatementBranch),
    CStyleFor {
        init: Box<Statement>,
        condition: Expression,
        update: Box<Statement>,
        body: Box<Statement>,
    },
    For {
        mutable: bool,
        pattern: Pattern,
        iterator: Expression,
        body: Box<Statement>,
    },
    Match(Expression, Vec<(Pattern, Block)>),

    Return(Option<Expression>),
    Break,
    Continue,

    Increment(Expression),
    Decrement(Expression),
}

#[derive(Debug, Clone, Serialize)]
pub struct VarDecl {
    pub mutable: bool,
    pub name: Pattern,
    pub type_: Option<TypeExpr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VarDeclStmt {
    pub decl: VarDecl,
    pub init: Option<Expression>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatementBranch {
    pub condition: Expression,
    pub body: Box<Statement>,
}
