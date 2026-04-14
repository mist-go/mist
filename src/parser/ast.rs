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
        export: bool,
        name: String,
        fields: ParamList,
    },
    FunctionDecl {
        export: bool,
        name: String,
        params: ParamList,
        return_type: Option<TypeExpr>,
        body: Block,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Statement {
    Expression(Expression),
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Expression {
    Identifier(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    StringLiteral(String),
    Postfix {
        initial: Box<Expression>,
        postfixes: Vec<Postfix>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Postfix {
    FieldAccess(String),
    FunctionCall(Vec<Expression>),
}

#[derive(Debug, Clone, Serialize)]
pub struct ParamList(pub Vec<(String, TypeExpr)>);

#[derive(Debug, Clone, Serialize)]
pub struct Block(pub Vec<Statement>);
