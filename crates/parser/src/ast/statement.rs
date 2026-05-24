use serde::Serialize;

use super::*;

#[derive(Debug, Clone, Serialize, Default)]
pub struct Block(
    pub Vec<Spanned<Expression>>,
    pub Option<Spanned<Expression>>,
);

#[derive(Debug, Clone, Serialize)]
pub enum StatementBody {
    Statement(Box<Statement>),
    Expression(Expression),
}

#[derive(Debug, Clone, Serialize)]
pub enum Statement {
    Block(Block),
    If {
        initial: StatementBranch,
        else_if: Vec<StatementBranch>,
        else_branch: Option<StatementBody>,
    },
    Loop(StatementBody),
    While(StatementBranch),
    CStyleFor {
        init: Expression,
        condition: Expression,
        update: Expression,
        body: StatementBody,
    },
    For {
        mutable: bool,
        pattern: Pattern,
        iterator: Expression,
        body: StatementBody,
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
    pub body: Box<StatementBody>,
}

impl Statement {
    pub fn is_block(&self) -> bool {
        match self {
            Self::Block(_) | Self::Match(_, _) => true,
            Self::While(branch) => branch.body.is_block(),
            Self::For { body, .. } | Self::Loop(body) | Self::CStyleFor { body, .. } => {
                body.is_block()
            }
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

impl StatementBody {
    pub fn is_block(&self) -> bool {
        match self {
            Self::Expression(_) => false,
            _ => true,
        }
    }
}
