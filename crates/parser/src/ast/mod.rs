use serde::Serialize;

pub mod expr;
pub mod statement;
pub mod top_level;

pub use expr::*;
pub use statement::*;
pub use top_level::*;

#[derive(Debug, Clone, Serialize)]
pub struct Path(pub Vec<Identifier>);

#[derive(Debug, Clone, Serialize)]
pub struct Identifier(pub String);

#[derive(Debug, Clone, Serialize, Default)]
pub struct ParamList(pub Vec<VarDecl>);

#[derive(Debug, Clone, Serialize)]
pub struct TypeExpr(pub TypeExprKind, pub Vec<TypePostfix>);

#[derive(Debug, Clone, Serialize)]
pub enum TypePostfix {
    Ref,
    RefMut,
    RefLifetime(Identifier),
    RefMutLifetime(Identifier),
    Dyn,
}

#[derive(Debug, Clone, Serialize)]
pub enum TypeExprKind {
    Path(Path, Option<Generics>),
    Tuple(Vec<TypeExpr>),
    Lifetime(Identifier),
}

#[derive(Debug, Clone, Serialize)]
pub struct Spanned<T> {
    pub line: usize,
    pub column: usize,
    pub item: T,
}

impl TypeExpr {
    pub fn no_px(kind: TypeExprKind) -> Self {
        Self(kind, Vec::new())
    }
}

impl From<GenericDecl> for Generic {
    fn from(value: GenericDecl) -> Self {
        match value {
            GenericDecl::Lifetime(life) => Generic::Lifetime(life),
            GenericDecl::Type(ty, _) => Generic::Type(TypeExpr(
                TypeExprKind::Path(Path(vec![ty]), None),
                Vec::new(),
            )),
        }
    }
}

impl<T> Spanned<T> {
    pub fn get_comment(&self) -> String {
        format!("/* {}:{} */", self.line, self.column)
    }
}
