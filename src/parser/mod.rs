use pest::Parser;
use pest_derive::Parser;

pub mod ast;
use ast::*;

#[derive(Parser)]
#[grammar = "./src/parser/grammar.pest"]
pub struct MistParser;

// convenience alias for pest errors
pub type ParseError = pest::error::Error<Rule>;

pub fn parse(source: &str) -> Result<Vec<TopLevel>, ParseError> {
    let pairs = MistParser::parse(Rule::program, source)?;

    let mut statements = vec![];

    // pairs is an iterator over the top-level program pair
    // we need to get its inner children
    for pair in pairs {
        match pair.as_rule() {
            Rule::program => {
                for inner in pair.into_inner() {
                    match inner.as_rule() {
                        Rule::import_decl => {
                            let path = inner.into_inner().next().unwrap().as_str().to_string();
                            statements.push(TopLevel::Import(path));
                        }
                        Rule::EOI => {}
                        _ => {}
                    }
                }
            }
            Rule::EOI => {}
            _ => {}
        }
    }

    Ok(statements)
}

impl Expression {
    pub fn from_pair(pair: pest::iterators::Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::expr => {
                let inner = pair.into_inner().next().unwrap();
                Expression::from_pair(inner)
            }
            Rule::identifier => Expression::Identifier(pair.as_str().to_string()),
            Rule::integer => {
                let value = pair.as_str().parse::<i64>().unwrap();
                Expression::IntLiteral(value)
            }
            Rule::float => {
                let value = pair.as_str().parse::<f64>().unwrap();
                Expression::FloatLiteral(value)
            }
            Rule::boolean => {
                let value = pair.as_str().parse::<bool>().unwrap();
                Expression::BoolLiteral(value)
            }
            Rule::string_lit => {
                let inner_str = pair.into_inner().next().unwrap().as_str();
                Expression::StringLiteral(inner_str.to_string())
            }
            _ => unimplemented!("Expression parsing not implemented yet"),
        }
    }
}
