use std::collections::HashSet;

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "./src/rev_mapper/grammar.pest"]
pub struct MistMapperParser;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MistMap(pub usize, pub usize);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RustMap(pub usize, pub usize);

pub fn get_mapping(input: &str) -> HashSet<(RustMap, MistMap)> {
    let mut mapping = HashSet::new();

    let pairs = MistMapperParser::parse(Rule::mapping, input)
        .unwrap()
        .next()
        .unwrap()
        .into_inner();

    for pair in pairs {
        if pair.as_rule() == Rule::map {
            let rs = pair.as_span().start_pos().line_col();
            let mut inner = pair.into_inner();

            let (line, col) = (
                inner.next().unwrap().as_str().parse().unwrap(),
                inner.next().unwrap().as_str().parse().unwrap(),
            );

            mapping.insert((RustMap(rs.0, rs.1), MistMap(line, col)));
        }
    }

    mapping
}

pub fn find_mapping(
    mapping: &HashSet<(RustMap, MistMap)>,
    target: &RustMap,
) -> Option<(RustMap, MistMap)> {
    mapping
        .iter()
        .copied()
        .filter(|(rust, _)| rust <= target)
        .max_by_key(|(rust, _)| *rust)
}
