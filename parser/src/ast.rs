use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FieldList(pub HashMap<String, (bool, TypeExpr)>);

#[derive(Debug, Clone, Serialize)]
pub struct ParamList(pub Vec<VarDecl>);

#[derive(Debug, Clone, Serialize)]
pub struct Block(pub Vec<Statement>);

#[derive(Debug, Clone, Serialize)]
pub enum TypePostfix {
    Ref,
    RefMut,
}

#[derive(Debug, Clone, Serialize)]
pub enum TypeExprKind {
    Path(StaticPath),
    PathParams(StaticPath, Vec<TypeExpr>),
    Tuple(Vec<TypeExpr>),
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeExpr(pub TypeExprKind, pub Vec<TypePostfix>);

#[derive(Debug, Clone, Serialize)]
pub struct StaticPath(pub Vec<String>);

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
    Include(StaticPath),
    StructDecl {
        export: bool,
        name: String,
        fields: FieldList,
    },
    FunctionDecl {
        export: bool,
        name: String,
        params: ParamList,
        return_type: TypeExpr,
        body: Block,
    },
}

#[derive(Debug, Clone, Serialize)]
pub enum Postfix {
    FieldAccess(String),
    Call(Vec<Expression>),
    MacroCall(String),
    StructCall(HashMap<String, Expression>),
    Index(Expression),
    Binary(BinaryOp, Expression),
}

#[derive(Debug, Clone, Serialize)]
pub enum Prefix {
    Ref,
    RefMut,
    Deref,
}

#[derive(Debug, Clone, Serialize)]
pub enum Statement {
    Expression(Expression),
    Block(Block),

    VarDecl(VarDeclStmt),
    VarAssign(VarAssignStmt),
    If(IfStmt),
    While(WhileStmt),
    For(ForStmt),

    Return(Option<Expression>),
    Break,
    Continue,
}

#[derive(Debug, Clone, Serialize)]
pub struct VarDecl {
    pub mutable: bool,
    pub name: String,
    pub type_: Option<TypeExpr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VarDeclStmt {
    pub decl: VarDecl,
    pub init: Option<Expression>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VarAssignStmt {
    pub target: Expression,
    pub value: Expression,
}

#[derive(Debug, Clone, Serialize)]
pub struct IfStmt {
    pub condition: Expression,
    pub then_branch: Box<Statement>,
    pub else_branch: Option<Box<Statement>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhileStmt {
    pub condition: Expression,
    pub body: Box<Statement>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForStmt {
    pub init: (bool, String, Option<Expression>),
    pub condition: Option<Expression>,
    pub update: Option<Box<Statement>>,
    pub body: Box<Statement>,
}

#[derive(Debug, Clone, Serialize)]
pub enum Expression {
    Path(StaticPath),
    IntLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    StringLiteral(String),
    TupleLiteral(Vec<Expression>),
    Fix {
        initial: Box<Expression>,
        prefixes: Vec<Prefix>,
        postfixes: Vec<Postfix>,
    },
}
