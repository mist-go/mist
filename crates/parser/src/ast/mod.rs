use serde::Serialize;

pub mod expr;
pub mod statement;
pub mod top_level;

pub use expr::*;
pub use statement::*;
pub use top_level::*;

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub struct Path(pub Vec<Identifier>);

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub struct Identifier(pub String);

#[derive(Debug, Clone, Serialize, Default)]
pub struct ParamList(pub Vec<VarDecl>);

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub enum TypeExpr {
    Ref {
        lifetime: Option<Identifier>,
        mutable: bool,
        ty: Box<TypeExpr>,
    },
    UnsafePtr {
        mutable: bool,
        ty: Box<TypeExpr>,
    },
    Dyn(Box<TypeExpr>),
    Path(Path, Option<Generics>),
    StaticFn(Vec<TypeExpr>, Option<Box<TypeExpr>>),
    Tuple(Vec<TypeExpr>),
    Lifetime(Identifier),
}

#[derive(Debug, Clone, Serialize)]
pub struct Spanned<T> {
    pub line: usize,
    pub column: usize,
    pub item: T,
}

impl From<GenericDecl> for Generic {
    fn from(value: GenericDecl) -> Self {
        match value {
            GenericDecl::Lifetime(life) => Generic::Lifetime(life),
            GenericDecl::Type(ty, _) => Generic::Type(TypeExpr::Path(Path(vec![ty]), None)),
        }
    }
}

impl Into<Option<Generics>> for GenericsDecl {
    fn into(self) -> Option<Generics> {
        if self.0.len() == 0 {
            None
        } else {
            Some(self.into())
        }
    }
}

impl From<GenericsDecl> for Generics {
    fn from(value: GenericsDecl) -> Self {
        Self(value.0.into_iter().map(Generic::from).collect())
    }
}
