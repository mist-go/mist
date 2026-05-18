use std::collections::HashSet;

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "./src/rev_mapper/grammar.pest"]
pub struct MistMapperParser;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct MistMap(pub usize, pub usize);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct RustMap(pub usize, pub usize);

pub fn get_mapping(input: &str) -> HashSet<(RustMap, MistMap)> {
    let mut mapping = HashSet::new();

    let pairs = MistMapperParser::parse(Rule::mapping, input).unwrap();

    for pair in pairs {
        if pair.as_rule() == Rule::map {
            let rs = pair.as_span().start_pos().line_col();
            let mut inner = pair.into_inner();

            mapping.insert((
                RustMap(rs.0, rs.1),
                MistMap(
                    inner.next().unwrap().as_str().parse().unwrap(),
                    inner.next().unwrap().as_str().parse().unwrap(),
                ),
            ));
        }
    }

    mapping
}
