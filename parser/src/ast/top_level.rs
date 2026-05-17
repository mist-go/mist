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
pub struct TopLevel(pub TopLevelKind, pub Vec<Attribute>);

#[derive(Debug, Clone, Serialize)]
pub enum TopLevelKind {
    ModAttribute,
    Import(Visibility, Path),
    Mod(Visibility, Identifier),
    ImplDecl(ImplDecl),
    TraitDecl {
        visibility: Visibility,
        name: Identifier,
        generics: GenericsDecl,
        requirements: Vec<TypeExpr>,
        items: Vec<FunctionDecl>,
    },
    EnumDecl {
        visibility: Visibility,
        name: Identifier,
        generics: GenericsDecl,
        fields: Vec<EnumItem>,
    },
    StructDecl {
        visibility: Visibility,
        name: Identifier,
        generics: GenericsDecl,
        fields: Vec<FieldDecl>,
    },
    FunctionDecl(FunctionDecl),
    ClassDecl {
        visibility: Visibility,
        name: Identifier,
        generics: GenericsDecl,
        fields: Vec<FieldDeclStmt>,
        constructor: ClassConstructor,
        items: Vec<ClassItem>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub enum ClassItem {
    Method(FunctionDecl),
    ImplDecl(ImplDecl),
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

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDecl {
    pub visibility: Visibility,
    pub name: Identifier,
    pub generics: GenericsDecl,
    pub params: ParamList,
    pub return_type: TypeExpr,
    pub body: Option<Block>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImplDecl {
    pub generics: GenericsDecl,
    pub target: TypeExpr,
    pub trait_: Option<TypeExpr>,
    pub methods: Vec<FunctionDecl>,
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
