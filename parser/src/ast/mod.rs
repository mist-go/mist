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
pub enum TypePostfix {
    Ref,
    RefMut,
    RefLifetime(Identifier),
    RefMutLifetime(Identifier),
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

impl TypeExpr {
    pub fn no_px(kind: TypeExprKind) -> Self {
        Self(kind, Vec::new())
    }
}
