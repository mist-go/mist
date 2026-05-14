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

#[macro_export]
macro_rules! ast_expr {
    ($inner:tt@$item:path { $($name:ident $t:tt $val:tt $($val2:tt)?),*}) => {{
        $(
            let $name = $crate::_ast_t!($t $crate::_ast_ti!($inner, $val $($val2)?));
        )*

        $item { $($name,)* }
    }};
}

#[macro_export]
macro_rules! _ast_t {
    (: $val:expr) => {
        $val.get()?
    };

    (? $val:expr) => {
        $val.or_else(|e| {
            if let Some(recovered) = e.recovered {
                Ok(recovered)
            } else {
                Err(e)
            }
        })
        .get()?
    };
}

#[macro_export]
macro_rules! _ast_ti {
    ($inner:ident, ! $val:ident) => {
        $val::try_from(&mut $inner)
    };

    ($inner:ident, @ $val:ident) => {
        $val::try_from($inner.next().unwrap())
    };

    ($inner:ident, ? $val:ident) => {
        $inner.next().map($val::try_from).transpose()
    };

    ($inner:ident, ? ($rule:path)) => {
        $crate::parser::listen_rule(&mut $inner, $rule)
    };

    ($inner:ident, & $val:expr) => {
        $val
    };
}
