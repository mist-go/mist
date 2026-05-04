use pest::Parser;
use pest_derive::Parser;

pub mod ast;

use ast::*;

#[derive(Parser)]
#[grammar = "./src/grammar.pest"]
pub struct MistParser;

// convenience alias for pest errors
pub type ParseError = pest::error::Error<Rule>;

pub fn parse(source: &str) -> Result<Vec<TopLevel>, ParseError> {
    let mut pairs = MistParser::parse(Rule::program, source)?;

    let mut statements = vec![];

    for pair in pairs.next().unwrap().into_inner() {
        if let Ok(stmt) = TopLevel::try_from(pair) {
            statements.push(stmt);
        }
    }

    Ok(statements)
}

impl From<pest::iterators::Pair<'_, Rule>> for TypeExpr {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        match pair.as_rule() {
            Rule::type_expr => TypeExpr::from(pair.into_inner().next().unwrap()),
            Rule::static_path => TypeExpr::Path(StaticPath::from(pair)),
            Rule::tuple_type => TypeExpr::Tuple(pair.into_inner().map(TypeExpr::from).collect()),
            _ => unimplemented!("{pair:#?}"),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for StaticPath {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        match pair.as_rule() {
            Rule::static_path => {
                StaticPath(pair.into_inner().map(|i| i.as_str().to_string()).collect())
            }
            _ => unimplemented!("{pair:#?}"),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for FieldList {
    fn from(pair: pest::iterators::Pair<Rule>) -> Self {
        let params = pair
            .into_inner()
            .map(|p| {
                let mut param_inner = p.into_inner();
                let export = if param_inner.peek().unwrap().as_rule() == Rule::export {
                    param_inner.next().unwrap();
                    true
                } else {
                    false
                };
                let param_type = TypeExpr::from(param_inner.next().unwrap());
                let param_name = param_inner.next().unwrap().as_str().to_string();
                (param_name, (export, param_type))
            })
            .collect();

        FieldList(params)
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for ParamList {
    fn from(pair: pest::iterators::Pair<Rule>) -> Self {
        let params = pair.into_inner().map(VarDecl::from).collect();

        ParamList(params)
    }
}

impl TryFrom<pest::iterators::Pair<'_, Rule>> for TopLevel {
    type Error = ();
    fn try_from(pair: pest::iterators::Pair<Rule>) -> Result<Self, ()> {
        match pair.as_rule() {
            Rule::import => {
                let path = pair.into_inner().next().unwrap().as_str().to_string();
                Ok(TopLevel::Import(path))
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

                let return_type = TypeExpr::from(inner.next().unwrap());

                let name = inner.next().unwrap().as_str().to_string();
                let params = if inner.peek().unwrap().as_rule() == Rule::param_list {
                    ParamList::from(inner.next().unwrap())
                } else {
                    ParamList(Vec::new())
                };

                let body = Block::from(inner.next().unwrap());

                Ok(TopLevel::FunctionDecl {
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
                let fields = FieldList::from(fields_pair);

                Ok(TopLevel::StructDecl {
                    export,
                    name,
                    fields,
                })
            }

            Rule::EOI => Err(()),
            _ => unimplemented!("TopLevel parsing not implemented yet {:?}", pair.as_rule()),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for Block {
    fn from(pair: pest::iterators::Pair<Rule>) -> Self {
        let statements = pair
            .into_inner()
            .flat_map(|pair| {
                if pair.as_rule() == Rule::statement_list {
                    pair.into_inner().map(Statement::from).collect()
                } else {
                    vec![Statement::from(pair)]
                }
            })
            .collect();
        Block(statements)
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for Statement {
    fn from(pair: pest::iterators::Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::statement => {
                let inner = pair.into_inner().next().unwrap();
                Statement::from(inner)
            }

            Rule::expr_stmt => {
                let expr_pair = pair.into_inner().next().unwrap();
                Statement::Expression(Expression::from(expr_pair))
            }

            Rule::block => Statement::Block(Block::from(pair.into_inner().next().unwrap())),

            Rule::var_decl_statement => Statement::VarDecl(VarDeclStmt::from(pair)),

            Rule::return_stmt => {
                let mut inner = pair.into_inner();

                let expr = inner.next().map(Expression::from);

                Statement::Return(expr)
            }

            Rule::break_stmt => Statement::Break,

            Rule::continue_stmt => Statement::Continue,

            Rule::if_stmt => {
                let mut inner = pair.into_inner();

                let condition = Expression::from(inner.next().unwrap());
                let then_branch = Statement::from(inner.next().unwrap());

                let else_branch = inner.next().map(Statement::from);

                Statement::If(IfStmt {
                    condition,
                    then_branch: Box::new(then_branch),
                    else_branch: else_branch.map(Box::new),
                })
            }

            Rule::while_stmt => {
                let mut inner = pair.into_inner();

                let condition = Expression::from(inner.next().unwrap());
                let body = Statement::from(inner.next().unwrap());

                Statement::While(WhileStmt {
                    condition,
                    body: Box::new(body),
                })
            }

            _ => unimplemented!(
                "Statement parsing not implemented yet: {:?}",
                pair.as_rule()
            ),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for Expression {
    fn from(pair: pest::iterators::Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::expr => {
                let mut inner = pair.into_inner();
                let exp = Expression::from(inner.next().unwrap());

                if inner.len() > 0 {
                    Expression::Postfix {
                        initial: Box::new(exp),
                        postfixes: inner.map(|p| Postfix::from(p)).collect(),
                    }
                } else {
                    exp
                }
            }
            Rule::primary => Expression::from(pair.into_inner().next().unwrap()),
            Rule::static_path => Expression::Path(StaticPath::from(pair)),
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

impl From<pest::iterators::Pair<'_, Rule>> for Postfix {
    fn from(pair: pest::iterators::Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::postfix => Postfix::from(pair.into_inner().next().unwrap()),

            Rule::field_px => {
                let field_name = pair.into_inner().next().unwrap().as_str().to_string();
                Postfix::FieldAccess(field_name)
            }

            Rule::call_px => Postfix::Call(pair.into_inner().map(Expression::from).collect()),

            Rule::struct_px => Postfix::StructCall(
                pair.into_inner()
                    .map(|p| {
                        let mut pi = p.into_inner();
                        (
                            pi.next().unwrap().as_str().to_string(),
                            Expression::from(pi.next().unwrap()),
                        )
                    })
                    .collect(),
            ),

            Rule::index_px => Postfix::Index(Expression::from(pair.into_inner().next().unwrap())),

            Rule::binary_px => {
                let mut inner = pair.into_inner();
                let op_pair = inner.next().unwrap();
                let op = match op_pair.as_str() {
                    "+" => BinaryOp::Plus,
                    "-" => BinaryOp::Minus,
                    "*" => BinaryOp::Multiply,
                    "/" => BinaryOp::Divide,
                    "%" => BinaryOp::Modulo,
                    "==" => BinaryOp::Equal,
                    "!=" => BinaryOp::NotEqual,
                    "<" => BinaryOp::LessThan,
                    ">" => BinaryOp::GreaterThan,
                    "<=" => BinaryOp::LessThanOrEqual,
                    ">=" => BinaryOp::GreaterThanOrEqual,

                    _ => {
                        unimplemented!("Binary operator not implemented yet: {}", op_pair.as_str())
                    }
                };
                Postfix::Binary(op, Expression::from(inner.next().unwrap()))
            }

            _ => unimplemented!("Postfix parsing not implemented yet {:?}", pair.as_rule()),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for VarDeclStmt {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        match pair.as_rule() {
            Rule::var_decl_statement => {
                let mut inner = pair.into_inner();

                let decl = VarDecl::from(inner.next().unwrap());

                let init = inner.next().map(Expression::from);

                VarDeclStmt { decl, init }
            }

            _ => unimplemented!(),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for VarDecl {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        match pair.as_rule() {
            Rule::var_decl => {
                let mut inner = pair.into_inner();

                let type_ = Some(inner.next().map(TypeExpr::from).unwrap());
                let mutable = if inner.peek().unwrap().as_rule() == Rule::mutable {
                    inner.next();
                    true
                } else {
                    false
                };
                let name = inner.next().unwrap().as_str().to_string();

                VarDecl {
                    mutable,
                    name,
                    type_,
                }
            }

            _ => unimplemented!("{:?}", pair.as_rule()),
        }
    }
}
