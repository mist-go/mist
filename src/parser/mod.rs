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

impl ParamList {
    pub fn from_pair(pair: pest::iterators::Pair<Rule>) -> Self {
        let params = pair
            .into_inner()
            .map(|p| {
                let mut param_inner = p.into_inner();
                let param_name = param_inner.next().unwrap().as_str().to_string();
                let param_type = TypeExpr::from_pair(param_inner.next().unwrap());
                (param_name, param_type)
            })
            .collect();

        ParamList(params)
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
                    ParamList::from_pair(params_pair)
                } else {
                    ParamList(vec![])
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

                let body = Block::from_pair(inner.next().unwrap());

                Some(TopLevel::FunctionDecl {
                    export,
                    name,
                    params,
                    return_type,
                    body,
                })
            }

            Rule::struct_decl => {
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
                let fields_pair = inner.next().unwrap();
                let fields = ParamList::from_pair(fields_pair);

                Some(TopLevel::StructDecl {
                    export,
                    name,
                    fields,
                })
            }

            Rule::EOI => None,
            _ => unimplemented!("TopLevel parsing not implemented yet {:?}", pair.as_rule()),
        }
    }
}

impl Block {
    pub fn from_pair(pair: pest::iterators::Pair<Rule>) -> Self {
        let statements = pair
            .into_inner()
            .flat_map(|pair| {
                if pair.as_rule() == Rule::statement_list {
                    pair.into_inner().map(Statement::from_pair).collect()
                } else {
                    vec![Statement::from_pair(pair)]
                }
            })
            .collect();
        Block(statements)
    }
}

impl Statement {
    pub fn from_pair(pair: pest::iterators::Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::statement => {
                let inner = pair.into_inner().next().unwrap();
                Statement::from_pair(inner)
            }

            Rule::expr_stmt => {
                let expr_pair = pair.into_inner().next().unwrap();
                Statement::Expression(Expression::from_pair(expr_pair))
            }

            Rule::block => Statement::Block(Block::from_pair(pair.into_inner().next().unwrap())),

            Rule::var_decl => {
                let mut inner = pair.into_inner();

                let kind_pair = inner.next().unwrap(); // let/const/var
                let name_pair = inner.next().unwrap(); // identifier

                let init = inner.next().map(|expr_pair| {
                    // expects "=" expr
                    Expression::from_pair(expr_pair.into_inner().next().unwrap())
                });

                let kind = match kind_pair.as_str() {
                    "let" => VarKind::Let,
                    "const" => VarKind::Const,
                    "var" => VarKind::Var,
                    _ => unreachable!(),
                };

                Statement::VarDecl {
                    kind,
                    name: name_pair.as_str().to_string(),
                    init,
                }
            }

            Rule::return_stmt => {
                let mut inner = pair.into_inner();

                let expr = inner.next().map(Expression::from_pair);

                Statement::Return(expr)
            }

            Rule::break_stmt => Statement::Break,

            Rule::continue_stmt => Statement::Continue,

            Rule::if_stmt => {
                let mut inner = pair.into_inner();

                let condition = Expression::from_pair(inner.next().unwrap());
                let then_branch = Statement::from_pair(inner.next().unwrap());

                let else_branch = inner.next().map(Statement::from_pair);

                Statement::If {
                    condition,
                    then_branch: Box::new(then_branch),
                    else_branch: else_branch.map(Box::new),
                }
            }

            Rule::while_stmt => {
                let mut inner = pair.into_inner();

                let condition = Expression::from_pair(inner.next().unwrap());
                let body = Statement::from_pair(inner.next().unwrap());

                Statement::While {
                    condition,
                    body: Box::new(body),
                }
            }

            Rule::for_stmt => {
                let mut inner = pair.into_inner();

                let init = inner.next().map(|p| match p.as_rule() {
                    Rule::var_decl_no_semicolon => {
                        let mut it = p.into_inner();

                        let kind = match it.next().unwrap().as_str() {
                            "let" => VarKind::Let,
                            "const" => VarKind::Const,
                            "var" => VarKind::Var,
                            _ => unreachable!(),
                        };

                        let name = it.next().unwrap().as_str().to_string();
                        let init_expr = it
                            .next()
                            .map(|e| Expression::from_pair(e.into_inner().next().unwrap()));

                        ForInit::VarDecl {
                            kind,
                            name,
                            init: init_expr,
                        }
                    }
                    _ => ForInit::Expr(Expression::from_pair(p)),
                });

                let condition = inner.next().map(Expression::from_pair);
                let update = inner.next().map(Expression::from_pair);
                let body = Statement::from_pair(inner.next().unwrap());

                Statement::For {
                    init,
                    condition,
                    update,
                    body: Box::new(body),
                }
            }

            _ => unimplemented!(
                "Statement parsing not implemented yet: {:?}",
                pair.as_rule()
            ),
        }
    }
}

impl Expression {
    pub fn from_pair(pair: pest::iterators::Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::expr => {
                let mut inner = pair.into_inner();
                let exp = Expression::from_pair(inner.next().unwrap());

                if inner.len() > 0 {
                    Expression::Postfix {
                        initial: Box::new(exp),
                        postfixes: inner.map(|p| Postfix::from_pair(p)).collect(),
                    }
                } else {
                    exp
                }
            }
            Rule::primary => Expression::from_pair(pair.into_inner().next().unwrap()),
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

            _ => unimplemented!(
                "Expression parsing not implemented yet {:?}",
                pair.as_rule()
            ),
        }
    }
}

impl Postfix {
    pub fn from_pair(pair: pest::iterators::Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::postfix => {
                let inner = pair.into_inner();
                if let Some(field) = inner.peek() {
                    if field.as_rule() == Rule::identifier {
                        Postfix::FieldAccess(field.as_str().to_string())
                    } else {
                        Postfix::FunctionCall(inner.map(|p| Expression::from_pair(p)).collect())
                    }
                } else {
                    Postfix::FunctionCall(vec![])
                }
            }
            _ => unimplemented!("Postfix parsing not implemented yet {:?}", pair.as_rule()),
        }
    }
}
