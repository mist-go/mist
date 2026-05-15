use crate::Rule;

pub type AstResult<'a, T, ET = T> = Result<T, AstError<'a, ET>>;

#[derive(Debug, Clone)]
pub enum ParseError<'a, T> {
    PreAst(pest::error::Error<Rule>),
    Ast(AstError<'a, T>),
}

#[derive(Debug, Clone)]
pub struct AstError<'a, T> {
    span: pest::Span<'a>,
    error_code: ErrorCode,
    error_message: String,
    recovered: Option<T>,
}

#[derive(Debug, Clone)]
pub enum ErrorCode {
    InvalidStatement = 200,
}

impl<T> From<pest::error::Error<Rule>> for ParseError<'_, T> {
    fn from(value: pest::error::Error<Rule>) -> Self {
        Self::PreAst(value)
    }
}

impl<'a, T> From<AstError<'a, T>> for ParseError<'a, T> {
    fn from(value: AstError<'a, T>) -> Self {
        Self::Ast(value)
    }
}

impl<'a, F> AstError<'a, F> {
    pub fn get<T>(self) -> AstError<'a, T> {
        AstError {
            span: self.span,
            error_code: self.error_code,
            error_message: self.error_message,
            recovered: None,
        }
    }
}

pub trait GetParseError<'a, F> {
    fn get<T>(self) -> AstResult<'a, F, T>;
}

impl<'a, F> GetParseError<'a, F> for AstResult<'a, F> {
    fn get<T>(self) -> AstResult<'a, F, T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.get()),
        }
    }
}

impl<'a, F> GetParseError<'a, Option<F>> for Result<Option<F>, AstError<'a, F>> {
    fn get<T>(self) -> AstResult<'a, Option<F>, T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.get()),
        }
    }
}

pub fn collect_recovered<'a, T, ET>(
    pairs: impl Iterator<Item = pest::iterators::Pair<'a, Rule>>,
) -> AstResult<'a, Vec<T>, Vec<T>>
where
    T: TryFrom<pest::iterators::Pair<'a, Rule>, Error = AstError<'a, ET>>,
{
    let mut items = Vec::new();
    let mut last_error: Option<AstError<'a, ET>> = None;

    for pair in pairs {
        match T::try_from(pair) {
            Ok(item) => items.push(item),
            Err(e) => {
                last_error = Some(e);
            }
        }
    }

    match last_error {
        Some(ast_err) => Err(AstError {
            span: ast_err.span,
            error_code: ast_err.error_code,
            error_message: ast_err.error_message,
            recovered: None,
        }),
        None => Ok(items),
    }
}
