use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FieldList(pub Vec<(String, bool, TypeExpr)>);

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
pub enum Attribute {
    /// #[test]
    Path(Path),

    /// #[name = "value"]
    NameValue { path: Path, value: Literal },

    /// #[derive(Clone, Copy)]
    List { path: Path, items: Vec<Attribute> },
}

#[derive(Debug, Clone, Serialize)]
pub enum TypeExprKind {
    Path(Path),
    PathParams(Path, Vec<TypeExpr>),
    Tuple(Vec<TypeExpr>),
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeExpr(pub TypeExprKind, pub Vec<TypePostfix>);

#[derive(Debug, Clone, Serialize)]
pub struct Path(pub Vec<String>);

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
pub struct TopLevel(pub TopLevelKind, pub Vec<Attribute>);

#[derive(Debug, Clone, Serialize)]
pub enum TopLevelKind {
    ModAttribute,
    Include(Path),
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
    StructCall(Vec<(String, Expression)>),
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
        pattern: String,
        iterator: Expression,
    },

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
pub struct StatementBranch {
    pub condition: Expression,
    pub body: Box<Statement>,
}

#[derive(Debug, Clone, Serialize)]
pub enum Expression {
    Literal(Literal),
    Path(Path),
    Fix {
        initial: Box<Expression>,
        prefixes: Vec<Prefix>,
        postfixes: Vec<Postfix>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub enum Literal {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Tuple(Vec<Expression>),
}
