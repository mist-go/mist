use crate::Rule;

pub type ParseResult<'a, T> = Result<T, ParseError<'a>>;

#[derive(Debug, Clone)]
pub enum ParseError<'a> {
    PreAst(pest::error::Error<Rule>),
    Ast {
        span: pest::Span<'a>,
        error_code: ErrorCode,
        error_message: String,
    },
}

#[derive(Debug, Clone)]
pub enum ErrorCode {
    InvalidStatement = 200,
}
