use std::fmt::Debug;

use pest::iterators::Pair;

use crate::Rule;

pub type AstResult<'a, T> = Result<T, AstError<'a>>;

#[derive(Debug, Clone)]
pub enum ParseError<'a> {
    PreAst(pest::error::Error<Rule>),
    Ast(AstError<'a>),
}

#[derive(Debug, Clone)]
pub struct AstError<'a> {
    pub span: pest::Span<'a>,
    pub error_code: ErrorCode,
    pub error_message: String,
}

#[derive(Debug, Clone)]
pub enum ErrorCode {
    InvalidStatement,
    AstGenBug,
    Module,
}

impl From<pest::error::Error<Rule>> for ParseError<'_> {
    fn from(value: pest::error::Error<Rule>) -> Self {
        Self::PreAst(value)
    }
}

impl<'a> From<AstError<'a>> for ParseError<'a> {
    fn from(value: AstError<'a>) -> Self {
        Self::Ast(value)
    }
}

impl<'a> AstError<'a> {
    #[track_caller]
    pub fn bug_unimplemented<T>(pair: Pair<'a, Rule>) -> AstResult<'a, T> {
        let loc = std::panic::Location::caller();

        Err(Self {
            span: pair.as_span(),
            error_code: ErrorCode::AstGenBug,
            error_message: format!(
                "Possible bug, unimplemented: {:#?}, at {}:{}",
                pair.as_rule(),
                loc.file(),
                loc.line(),
            ),
        })
    }
}

pub trait IntoErr<T, FA, FR> {
    fn get(self) -> T;
    fn get_map(self, m: impl Fn(FA) -> FR) -> T;
}

pub trait GetLength {
    fn len(&self) -> usize;
}

impl<T, E> GetLength for Result<Vec<T>, E> {
    fn len(&self) -> usize {
        if let Ok(v) = self { v.len() } else { 0 }
    }
}

pub fn collect_recovered<'a, T: Debug>(
    pairs: impl Iterator<Item = pest::iterators::Pair<'a, Rule>>,
) -> AstResult<'a, Vec<T>>
where
    T: TryFrom<pest::iterators::Pair<'a, Rule>, Error = AstError<'a>>,
{
    collect_recovered_map(pairs, T::try_from)
}

pub fn collect_recovered_map<'a, T: Debug, F>(
    pairs: impl Iterator<Item = pest::iterators::Pair<'a, Rule>>,
    f: F,
) -> AstResult<'a, Vec<T>>
where
    F: Fn(pest::iterators::Pair<'a, Rule>) -> AstResult<'a, T>,
{
    let mut items = Vec::new();
    let mut last_error: Option<AstError<'a>> = None;

    for pair in pairs {
        match f(pair) {
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
        }),
        None => Ok(items),
    }
}
