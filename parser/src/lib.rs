use pest::Parser;
use pest_derive::Parser;

pub mod ast;
pub mod error;
pub mod parser;

use ast::*;

use crate::error::{GetParseError, ParseError};

#[derive(Parser)]
#[grammar = "./src/grammar.pest"]
pub struct MistParser;

pub fn parse<'a>(source: &'a str) -> Result<Vec<TopLevel>, ParseError<'a, Vec<TopLevel>>> {
    let mut pairs = MistParser::parse(Rule::program, source)?;

    let mut statements = vec![];

    for pair in pairs.next().unwrap().into_inner() {
        if pair.as_rule() != Rule::EOI {
            statements.push(TopLevel::try_from(pair).get()?);
        }
    }

    Ok(statements)
}
