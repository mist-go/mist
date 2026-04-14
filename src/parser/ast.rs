use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum TypeExpr {
    Identifier(String),
}

#[derive(Debug, Clone, Serialize)]
pub enum TopLevel {
    Import(String),
    StructDecl {
        name: String,
        fields: Vec<(String, TypeExpr)>,
    },
    FunctionDecl {
        name: String,
        params: Vec<(String, TypeExpr)>,
        return_type: Option<TypeExpr>,
        body: Vec<Statement>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub enum Statement {
    Expression(String),
}

#[derive(Debug, Clone, Serialize)]
pub enum Expression {
    Identifier(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    StringLiteral(String),
    FunctionCall {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
}
