use crate::Rule;

pub type ParseResult<'a, T, ET = T> = Result<T, ParseError<'a, ET>>;

#[derive(Debug, Clone)]
pub enum ParseError<'a, T> {
    PreAst(pest::error::Error<Rule>),
    Ast {
        span: pest::Span<'a>,
        error_code: ErrorCode,
        error_message: String,
        recovered: Option<T>,
    },
}

#[derive(Debug, Clone)]
pub enum ErrorCode {
    InvalidStatement = 200,
}

impl<'a, F> ParseError<'a, F> {
    pub fn get<T>(self) -> ParseError<'a, T> {
        match self {
            Self::Ast {
                span,
                error_code,
                error_message,
                ..
            } => ParseError::Ast {
                span,
                error_code,
                error_message,
                recovered: None,
            },
            Self::PreAst(pest_err) => ParseError::PreAst(pest_err),
        }
    }
}

impl<T> From<pest::error::Error<Rule>> for ParseError<'_, T> {
    fn from(value: pest::error::Error<Rule>) -> Self {
        Self::PreAst(value)
    }
}

pub trait GetParseError<'a, F> {
    fn get<T>(self) -> ParseResult<'a, F, T>;
}

impl<'a, F> GetParseError<'a, F> for ParseResult<'a, F> {
    fn get<T>(self) -> ParseResult<'a, F, T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.get()),
        }
    }
}
