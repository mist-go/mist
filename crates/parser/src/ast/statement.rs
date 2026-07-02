use serde::Serialize;

use super::*;

#[derive(Debug, Clone, Serialize, Default)]
pub struct Block {
    pub statements: Vec<Spanned<Expression>>,
    pub soft_return: Option<Spanned<Expression>>,
}

#[derive(Debug, Clone, Serialize)]
pub enum Statement {
    UnsafeBlock(Block),
    Block(Block),
    TopLevel(Box<TopLevel>),
    If {
        initial: StatementBranch,
        else_if: Vec<StatementBranch>,
        else_branch: Option<Block>,
    },
    Loop(Block),
    While(StatementBranch),
    CStyleFor {
        init: Expression,
        condition: Expression,
        update: Expression,
        body: Block,
    },
    For {
        pattern: Pattern,
        iterator: Expression,
        body: Block,
    },
    Match(Expression, Vec<Spanned<MatchItem>>),

    VarDecl(VarDeclStmt),
    Return(Option<Expression>),
    Break,
    Continue,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchItem(pub Vec<Pattern>, pub Expression);

#[derive(Debug, Clone, Serialize)]
pub struct VarDecl {
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
    pub body: Box<Block>,
}

impl Statement {
    pub fn is_block(&self) -> bool {
        match self {
            Self::VarDecl(_) | Self::Return(_) | Self::Break | Self::Continue => false,
            _ => true,
        }
    }
}
