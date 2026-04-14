use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum TypeExpr {
    Identifier(String),
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum TopLevel {
    Import(String),
    StructDecl {
        name: String,
        fields: ParamList,
    },
    FunctionDecl {
        name: String,
        params: ParamList,
        return_type: Option<TypeExpr>,
        body: Vec<Statement>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Statement {
    Expression(String),
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value")]
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

#[derive(Debug, Clone, Serialize)]
pub struct ParamList(pub Vec<(String, TypeExpr)>);
