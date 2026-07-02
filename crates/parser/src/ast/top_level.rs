use serde::Serialize;

use super::*;

#[derive(Debug, Clone, Serialize)]
pub enum Visibility {
    Public,
    PublicTarget(Path),
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
pub struct TopLevel(pub Spanned<TopLevelKind>, pub Vec<Attribute>);

#[derive(Debug, Clone, Serialize)]
pub enum TopLevelKind {
    ModAttribute,
    Import(Visibility, Path),
    DeclareModule(Visibility, Identifier),
    ImplDecl(ImplDecl),
    StaticDecl(VarDeclStmt),
    ConstDecl(VarDeclStmt),
    TraitDecl {
        visibility: Visibility,
        name: Identifier,
        generics: GenericsDecl,
        requirements: Vec<TypeExpr>,
        items: Vec<Spanned<FunctionDecl>>,
    },
    EnumDecl {
        visibility: Visibility,
        name: Identifier,
        generics: GenericsDecl,
        fields: Vec<Spanned<EnumItem>>,
    },
    StructDecl {
        visibility: Visibility,
        name: Identifier,
        generics: GenericsDecl,
        fields: Vec<Spanned<FieldDecl>>,
    },
    FunctionDecl(FunctionDecl),
    ClassDecl {
        visibility: Visibility,
        name: Identifier,
        generics: GenericsDecl,
        inherits: Option<ExprPath>,
        fields: Vec<Spanned<FieldDeclStmt>>,
        constructor: Option<Spanned<ClassConstructor>>,
        items: Vec<ClassItem>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub enum ClassItem {
    Method(Spanned<FunctionDecl>),
    ImplDecl(Spanned<ImplDecl>),
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct GenericsDecl(pub Vec<GenericDecl>);

#[derive(Debug, Clone, Serialize)]
pub enum GenericDecl {
    Lifetime(Identifier),
    Type(Identifier, Vec<TypeExpr>),
}

#[derive(Debug, Clone, Serialize)]
pub enum EnumItem {
    Named(Identifier),
    Tuple(Identifier, Vec<TypeExpr>),
    Struct(Identifier, Vec<FieldDecl>),
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassConstructor {
    pub visibility: Visibility,
    pub generics: GenericsDecl,
    pub params: ParamList,
    pub body: Block,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub struct Override(pub Option<ExprPath>);

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDecl {
    pub visibility: Visibility,
    pub is_override: Option<Override>,
    pub name: Identifier,
    pub generics: GenericsDecl,
    pub self_param: Option<(bool, Option<Identifier>, bool)>,
    pub params: ParamList,
    pub return_type: Option<TypeExpr>,
    pub body: Option<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImplDecl {
    pub generics: GenericsDecl,
    pub target: TypeExpr,
    pub trait_: Option<TypeExpr>,
    pub methods: Vec<Spanned<FunctionDecl>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldDecl {
    pub visibility: Visibility,
    pub type_: TypeExpr,
    pub name: Identifier,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldDeclStmt {
    pub decl: FieldDecl,
    pub init: Option<Expression>,
}
