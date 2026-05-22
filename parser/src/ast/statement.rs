use serde::Serialize;

use super::*;

#[derive(Debug, Clone, Serialize, Default)]
pub struct Block(
    pub Vec<Spanned<Expression>>,
    pub Option<Spanned<Expression>>,
);

#[derive(Debug, Clone, Serialize)]
pub enum Statement {
    Block(Block),
    If {
        initial: StatementBranch,
        else_if: Vec<StatementBranch>,
        else_branch: Option<Box<Statement>>,
    },
    While(StatementBranch),
    CStyleFor {
        init: Expression,
        condition: Expression,
        update: Expression,
        body: Expression,
    },
    For {
        mutable: bool,
        pattern: Pattern,
        iterator: Expression,
        body: Box<Statement>,
    },
    Match(Expression, Vec<(Vec<Pattern>, Expression)>),

    VarDecl(VarDeclStmt),
    Return(Option<Expression>),
    Break,
    Continue,
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

impl Statement {
    pub fn is_block(&self) -> bool {
        match self {
            Self::VarDecl(_) | Self::Return(_) | Self::Break | Self::Continue => false,
            _ => true,
        }
    }
}
