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
    let mut pairs = MistParser::parse(Rule::program, source)?;

    let mut statements = vec![];

    for pair in pairs.next().unwrap().into_inner() {
        if let Some(stmt) = TopLevel::from_pair(pair) {
            statements.push(stmt);
        }
    }

    Ok(statements)
}

impl TypeExpr {
    pub fn from_pair(pair: pest::iterators::Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::type_expr => {
                let inner = pair.into_inner().next().unwrap();
                TypeExpr::from_pair(inner)
            }
            Rule::identifier => TypeExpr::Identifier(pair.as_str().to_string()),
            _ => unimplemented!("TypeExpr parsing not implemented yet"),
        }
    }
}

impl TopLevel {
    pub fn from_pair(pair: pest::iterators::Pair<Rule>) -> Option<Self> {
        match pair.as_rule() {
            Rule::import => {
                let path = pair.into_inner().next().unwrap().as_str().to_string();
                Some(TopLevel::Import(path))
            }
            Rule::function_decl => {
                let mut inner = pair.into_inner();
                let export = if let Some(first) = inner.peek() {
                    if first.as_rule() == Rule::export {
                        inner.next();
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                let name = inner.next().unwrap().as_str().to_string();
                let params_pair = inner.next().unwrap();
                let params = if params_pair.as_rule() == Rule::param_list {
                    params_pair
                        .into_inner()
                        .map(|p| {
                            let mut param_inner = p.into_inner();
                            let param_name = param_inner.next().unwrap().as_str().to_string();
                            let param_type = TypeExpr::from_pair(param_inner.next().unwrap());
                            (param_name, param_type)
                        })
                        .collect()
                } else {
                    vec![]
                };
                let return_type = if let Some(next) = inner.peek() {
                    if next.as_rule() == Rule::type_expr {
                        Some(TypeExpr::from_pair(inner.next().unwrap()))
                    } else {
                        None
                    }
                } else {
                    None
                };

                // For now, we'll just ignore the function body and return an empty vector
                Some(TopLevel::FunctionDecl {
                    name,
                    params,
                    return_type,
                    body: vec![],
                })
            }

            Rule::EOI => None,
            _ => unimplemented!("TopLevel parsing not implemented yet {:?}", pair.as_rule()),
        }
    }
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
