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
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::type_expr => TypeExpr(
                TypeExprKind::from(inner.next().unwrap()),
                inner.map(TypePostfix::from).collect(),
            ),
            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for TypePostfix {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        let rule = pair.as_rule();
        let inner = pair.into_inner();

        match rule {
            Rule::ref_type => {
                if inner.peek().is_some() {
                    TypePostfix::RefMut
                } else {
                    TypePostfix::Ref
                }
            }
            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for TypeExprKind {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::tuple_type => TypeExprKind::Tuple(inner.map(TypeExpr::from).collect()),
            Rule::path_type => {
                let path = StaticPath::from(inner.next().unwrap());
                let params = inner.map(TypeExpr::from).collect::<Vec<_>>();

                if params.len() == 0 {
                    TypeExprKind::Path(path)
                } else {
                    TypeExprKind::PathParams(path, params)
                }
            }
            _ => unimplemented!("{rule:#?}"),
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
        ParamList(pair.into_inner().map(VarDecl::from).collect())
    }
}

impl TryFrom<pest::iterators::Pair<'_, Rule>> for TopLevel {
    type Error = ();
    fn try_from(pair: pest::iterators::Pair<Rule>) -> Result<Self, ()> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::import => Ok(TopLevel::Include(StaticPath::from(inner.next().unwrap()))),

            Rule::function_decl => {
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
            _ => unimplemented!("{rule:#?}"),
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
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::statement => Statement::from(inner.next().unwrap()),

            Rule::expr_stmt => Statement::Expression(Expression::from(inner.next().unwrap())),

            Rule::block => Statement::Block(Block::from(inner.next().unwrap())),

            Rule::var_decl_statement => Statement::VarDecl(VarDeclStmt::from(pair)),

            Rule::return_stmt => {
                let expr = inner.next().map(Expression::from);

                Statement::Return(expr)
            }

            Rule::break_stmt => Statement::Break,

            Rule::continue_stmt => Statement::Continue,

            Rule::if_stmt => {
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
                let condition = Expression::from(inner.next().unwrap());
                let body = Statement::from(inner.next().unwrap());

                Statement::While(WhileStmt {
                    condition,
                    body: Box::new(body),
                })
            }

            Rule::assign_statement => Statement::VarAssign(VarAssignStmt {
                target: Expression::from(inner.next().unwrap()),
                value: Expression::from(inner.next().unwrap()),
            }),

            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for Expression {
    fn from(pair: pest::iterators::Pair<Rule>) -> Self {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::expr => {
                let mut prefixes = Vec::new();

                while inner
                    .peek()
                    .map(|v| v.as_rule() == Rule::prefix)
                    .unwrap_or_default()
                {
                    prefixes.push(Prefix::from(inner.next().unwrap()));
                }

                let exp = Expression::from(inner.next().unwrap());

                if inner.len() > 0 || prefixes.len() > 0 {
                    Expression::Fix {
                        initial: Box::new(exp),
                        prefixes,
                        postfixes: inner.map(|p| Postfix::from(p)).collect(),
                    }
                } else {
                    exp
                }
            }
            Rule::primary => Expression::from(inner.next().unwrap()),
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
            Rule::tuple => Expression::TupleLiteral(inner.map(Expression::from).collect()),
            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for Prefix {
    fn from(pair: pest::iterators::Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::prefix => Self::from(pair.into_inner().next().unwrap()),
            Rule::deref_px => Self::Deref,
            Rule::mut_ref_px => Self::RefMut,
            Rule::ref_px => Self::Ref,
            _ => unimplemented!("{pair:#?}"),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for Postfix {
    fn from(pair: pest::iterators::Pair<Rule>) -> Self {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::postfix => Postfix::from(inner.next().unwrap()),

            Rule::field_px => {
                let field_name = inner.next().unwrap().as_str().to_string();
                Postfix::FieldAccess(field_name)
            }

            Rule::call_px => Postfix::Call(inner.map(Expression::from).collect()),

            Rule::struct_px => Postfix::StructCall(
                inner
                    .map(|p| {
                        let mut pi = p.into_inner();
                        (
                            pi.next().unwrap().as_str().to_string(),
                            Expression::from(pi.next().unwrap()),
                        )
                    })
                    .collect(),
            ),

            Rule::index_px => Postfix::Index(Expression::from(inner.next().unwrap())),

            Rule::binary_px => {
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

            Rule::macro_call_px => Postfix::MacroCall(inner.as_str().to_string()),

            _ => unimplemented!("{rule:#?}"),
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
