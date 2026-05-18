pub mod common;
pub mod items;

use crate::{Rule, ast::Spanned};

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

impl<'a, T: TryFrom<pest::iterators::Pair<'a, Rule>>> TryFrom<pest::iterators::Pair<'a, Rule>>
    for Spanned<T>
{
    type Error = T::Error;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let span = pair.as_span().start_pos().line_col();

        Ok(Self {
            line: span.0,
            column: span.1,
            item: pair.try_into()?,
        })
    }
}
