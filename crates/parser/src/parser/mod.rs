pub mod common;
pub mod items;

use crate::{
    Rule,
    ast::Spanned,
    error::{AstError, AstResult},
};

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

pub fn consume_rule_map<'a, T>(
    pairs: &mut pest::iterators::Pairs<'a, Rule>,
    rule: Rule,
    map: impl Fn(pest::iterators::Pair<'a, Rule>) -> AstResult<'a, T>,
) -> AstResult<'a, Option<T>> {
    let pair = consume_rule(pairs, rule);

    if let Some(pair) = pair {
        Some(map(pair)).transpose()
    } else {
        Ok(None)
    }
}

impl<T> Spanned<T> {
    fn new_pair(pair: pest::iterators::Pair<'_, Rule>, item: T) -> Self {
        let span = pair.as_span().start_pos().line_col();

        Self {
            line: span.0,
            column: span.1,
            item,
        }
    }
}

impl<'a, T: TryFrom<pest::iterators::Pair<'a, Rule>, Error = AstError<'a>>>
    TryFrom<pest::iterators::Pair<'a, Rule>> for Spanned<T>
{
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let span = pair.as_span().start_pos().line_col();

        Ok(Self {
            line: span.0,
            column: span.1,
            item: pair.try_into()?,
        })
    }
}
