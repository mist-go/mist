#[derive(Debug, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<TopLevel>,
}

#[derive(Debug, Clone)]
pub enum TopLevel {
    Function(Function),
    Struct(Struct),
    Class(Class),
    Import(Import),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_expr: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<StructField>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub type_expr: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub fields: Vec<StructField>,
    pub methods: Vec<Function>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Let(LetStatement),
    Return(ReturnStatement),
    Expression(Expression),
    If(IfStatement),
    For(ForStatement),
}

#[derive(Debug, Clone)]
pub struct LetStatement {
    pub name: String,
    pub type_expr: Option<TypeExpr>,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub value: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfStatement {
    pub condition: Expression,
    pub body: Vec<Statement>,
    pub else_body: Option<Vec<Statement>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ForStatement {
    pub var: String,
    pub iterator: Expression,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Identifier(String, Span),
    Integer(i64, Span),
    Float(f64, Span),
    StringLit(String, Span),
    Bool(bool, Span),
    BinaryOp(Box<BinaryOp>),
    UnaryOp(Box<UnaryOp>),
    Call(Box<CallExpr>),
    FieldAccess(Box<FieldAccess>),
    StructInit(Box<StructInit>),
}

#[derive(Debug, Clone)]
pub struct BinaryOp {
    pub left: Expression,
    pub op: BinOperator,
    pub right: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum BinOperator {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub struct UnaryOp {
    pub op: UnaryOperator,
    pub expr: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: Expression,
    pub args: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FieldAccess {
    pub object: Expression,
    pub field: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructInit {
    pub name: String,
    pub fields: Vec<(String, Expression)>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Named(String),
    Array(Box<TypeExpr>),
    Optional(Box<TypeExpr>),
}
