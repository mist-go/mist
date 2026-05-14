pub mod common;
pub mod items;

use crate::{Rule, error::AstResult};

pub fn listen_rule<'a>(
    pairs: &mut pest::iterators::Pairs<'_, Rule>,
    rule: Rule,
) -> AstResult<'a, bool> {
    let consumed = pairs
        .peek()
        .map(|p| p.as_rule() == rule)
        .unwrap_or_default();

    if consumed {
        pairs.next();
    }

    Ok(consumed)
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
