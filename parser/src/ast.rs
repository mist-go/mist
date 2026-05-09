use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Identifier(pub String);

#[derive(Debug, Clone, Serialize, Default)]
pub struct FieldList(pub Vec<(Identifier, Visibility, TypeExpr)>);

#[derive(Debug, Clone, Serialize, Default)]
pub struct ParamList(pub Vec<VarDecl>);

#[derive(Debug, Clone, Serialize, Default)]
pub struct Block(pub Vec<Statement>);

#[derive(Debug, Clone, Serialize)]
pub enum TypePostfix {
    Ref,
    RefMut,
    RefLifetime(Identifier),
    RefMutLifetime(Identifier),
}

#[derive(Debug, Clone, Serialize)]
pub enum Visibility {
    Public,
    Private,
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
    Lifetime(Identifier),
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeExpr(pub TypeExprKind, pub Vec<TypePostfix>);

#[derive(Debug, Clone, Serialize)]
pub struct Path(pub Vec<Identifier>);

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
    And,
    Or,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopLevel(pub TopLevelKind, pub Vec<Attribute>);

#[derive(Debug, Clone, Serialize)]
pub enum TopLevelKind {
    ModAttribute,
    Import(Path),
    Mod(Identifier),
    EnumDecl {
        visibility: Visibility,
        name: Identifier,
        generics: Generics,
        fields: Vec<EnumItem>,
    },
    StructDecl {
        visibility: Visibility,
        generics: Generics,
        name: Identifier,
        fields: FieldList,
    },
    FunctionDecl(FunctionDecl),
    ClassDecl {
        visibility: Visibility,
        name: Identifier,
        generics: Generics,
        fields: Vec<VarDeclStmt>,
        constructor: ClassConstructor,
        methods: Vec<FunctionDecl>,
    },
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Generics(pub Vec<Generic>);

#[derive(Debug, Clone, Serialize)]
pub enum Generic {
    Lifetime(Identifier),
    Type(Identifier, Vec<TypeExpr>),
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
pub enum EnumItem {
    Named(Identifier),
    Tuple(Identifier, Vec<TypeExpr>),
    Struct(Identifier, FieldList),
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassConstructor {
    pub visibility: Visibility,
    pub generics: Generics,
    pub params: ParamList,
    pub body: Block,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDecl {
    pub visibility: Visibility,
    pub name: Identifier,
    pub generics: Generics,
    pub params: ParamList,
    pub return_type: TypeExpr,
    pub body: Block,
}

#[derive(Debug, Clone, Serialize)]
pub enum Postfix {
    FieldAccess(Identifier),
    Call(Vec<Expression>),
    MacroCall(String),
    StructCall(Vec<(Identifier, Expression)>),
    Index(Expression),
    Binary(BinaryOp, Expression),
}

#[derive(Debug, Clone, Serialize)]
pub enum Prefix {
    Ref,
    RefMut,
    Deref,
    New,
    Not,
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
        mutable: bool,
        pattern: Pattern,
        iterator: Expression,
        body: Box<Statement>,
    },
    Match(Expression, Vec<(Pattern, Block)>),

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
