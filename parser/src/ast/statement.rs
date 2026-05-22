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
        else_branch: Option<Box<Expression>>,
    },
    Loop(Expression),
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
        body: Box<Expression>,
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
    pub body: Box<Expression>,
}

impl Statement {
    pub fn is_block(&self) -> bool {
        match self {
            Self::Block(_) => true,
            Self::If {
                initial,
                else_if,
                else_branch,
            } => {
                else_branch
                    .as_ref()
                    .map(|v| v.is_block())
                    .unwrap_or_default()
                    || else_if
                        .last()
                        .map(|b| b.body.is_block())
                        .unwrap_or_default()
                    || initial.body.is_block()
            }
            _ => false,
        }
    }
}
