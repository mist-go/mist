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
    ($($item:ident)::+ { $($k:ident: $v:expr),* $(,)? }) => {{
        let mut err = None;

        let v = $($item)::+ {
            $(
                $k: {
                    let r = $v.get();

                    if let Err(e) = r {
                        err = Some(e.clone().get());

                        if let Some(recovered) = e.recovered {
                            Ok(recovered)
                        } else {
                            Err(e)
                        }
                    } else {
                        r
                    }
                }.get()?
            ),*
        };

        if let Some(mut e) = err {
            e.recovered = Some(v);
            Err(e)
        } else {
            Ok(v)
        }
    }};

    ($($item:ident)::+ ( $($v:expr),* $(,)? )) => {{
        let mut err = None;

        let v = $($item)::+ ( $({
            let r = $v.get();

            if let Err(e) = r {
                err = Some(e.clone().get());

                if let Some(recovered) = e.recovered {
                    Ok(recovered)
                } else {
                    Err(e)
                }
            } else {
                r
            }
        }.get()?),* );

        if let Some(mut e) = err {
            e.recovered = Some(v);
            Err(e)
        } else {
            Ok(v)
        }
    }};

    ($($item:ident)::+) => {
        $($item)::+
    };
}
