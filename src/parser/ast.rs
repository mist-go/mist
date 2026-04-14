#[derive(Debug, Clone)]
pub enum TypeExpr {
    Identifier(String),
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum Statement {
    Expression(String),
}

pub enum Expression {
    Identifier(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    StringLiteral(String),
}
