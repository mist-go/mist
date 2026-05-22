pub mod common;
pub mod items;

use crate::{Rule, ast::Spanned, error::AstError};

pub fn listen_rule(pairs: &mut pest::iterators::Pairs<'_, Rule>, rule: Rule) -> bool {
    let consumed = pairs
        .peek()
        .map(|p| p.as_rule() == rule)
        .unwrap_or_default();

    if consumed {
        pairs.next();
    }

    consumed
}

pub fn consume_rule<'a>(
    pairs: &mut pest::iterators::Pairs<'a, Rule>,
    rule: Rule,
) -> Option<pest::iterators::Pair<'a, Rule>> {
    let consumed = pairs
        .peek()
        .map(|p| p.as_rule() == rule)
        .unwrap_or_default();

    if consumed { pairs.next() } else { None }
}

impl<'a, T: TryFrom<pest::iterators::Pair<'a, Rule>, Error = AstError<'a, T>>>
    TryFrom<pest::iterators::Pair<'a, Rule>> for Spanned<T>
{
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let span = pair.as_span().start_pos().line_col();

        Ok(Self {
            line: span.0,
            column: span.1,
            item: pair.try_into().map_err(|err: AstError<'_, T>| AstError {
                span: err.span,
                error_code: err.error_code,
                error_message: err.error_message,
                recovered: err.recovered.map(|v| Spanned {
                    line: span.0,
                    column: span.1,
                    item: v,
                }),
            })?,
        })
    }
}
