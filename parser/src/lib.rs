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

    let mut analyzer = error::AstErrorAnalyzer(None);

    for pair in pairs.next().unwrap().into_inner() {
        if pair.as_rule() != Rule::EOI {
            statements.push(analyzer.get(TopLevel::try_from(pair)).get()?);
        }
    }

    match analyzer.build(statements) {
        Ok(v) => Ok(v),
        Err(e) => Err(ParseError::Ast(e)),
    }
}

#[macro_export]
macro_rules! ast_expr {
    ($($item:ident)::+ { $($k:ident: $v:expr),* $(,)? }) => {{
        let mut analyzer = $crate::error::AstErrorAnalyzer(None);

        let v = $($item)::+ { $($k: analyzer.get($v).get()?),* };

        analyzer.build(v)
    }};

    ($($item:ident)::+ ( $($v:expr),* $(,)? )) => {{
        let mut analyzer = $crate::error::AstErrorAnalyzer(None);

        let v = $($item)::+ ( $(analyzer.get($v).get()?),* );

        analyzer.build(v)
    }};

    ($($item:ident)::+) => {
        $($item)::+
    };
}
