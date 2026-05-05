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
        if pair.as_rule() != Rule::EOI {
            statements.push(TopLevel::from(pair));
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
                let path = Path::from(inner.next().unwrap());
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

impl From<pest::iterators::Pair<'_, Rule>> for Path {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        match pair.as_rule() {
            Rule::static_path => Path(pair.into_inner().map(|i| i.as_str().to_string()).collect()),
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
                (param_name, export, param_type)
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

impl From<pest::iterators::Pair<'_, Rule>> for Attribute {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        match pair.as_rule() {
            Rule::attribute => {
                // unwrap #[ ... ]
                Attribute::from(pair.into_inner().next().unwrap())
            }

            Rule::meta => {
                let mut inner = pair.into_inner();

                // first item is always the path
                let path = Path::from(inner.next().unwrap());

                // check what comes next
                match inner.next() {
                    None => {
                        // #[path]
                        Attribute::Path(path)
                    }

                    Some(next) => match next.as_rule() {
                        Rule::primary => {
                            // #[path = literal]
                            Attribute::NameValue {
                                path,
                                value: Literal::from(next),
                            }
                        }

                        Rule::meta_list => {
                            // #[path(...)]
                            let items = next.into_inner().map(Attribute::from).collect();

                            Attribute::List { path, items }
                        }

                        _ => unreachable!("unexpected rule in meta: {:?}", next.as_rule()),
                    },
                }
            }

            Rule::meta_list => {
                // This case usually won't be hit directly,
                // but it's nice to keep it safe if reused
                let items = pair.into_inner().map(Attribute::from).collect::<Vec<_>>();

                // NOTE: this shouldn't normally construct an Attribute alone
                // but you can panic or wrap depending on your design
                panic!("meta_list should be handled inside meta: {:?}", items);
            }

            _ => unreachable!("unexpected rule: {:?}", pair.as_rule()),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for TopLevel {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        let mut inner = pair.into_inner();

        let attributes = inner
            .next()
            .unwrap()
            .into_inner()
            .map(Attribute::from)
            .collect::<Vec<_>>();

        TopLevel(
            inner
                .next()
                .map(TopLevelKind::from)
                .unwrap_or(TopLevelKind::ModAttribute),
            attributes,
        )
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for StatementBranch {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        let mut inner = pair.into_inner();

        let condition = Expression::from(inner.next().unwrap());
        let body = Statement::from(inner.next().unwrap());

        StatementBranch {
            condition,
            body: Box::new(body),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for ClassConstructor {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
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

        let params = if inner.peek().unwrap().as_rule() == Rule::param_list {
            ParamList::from(inner.next().unwrap())
        } else {
            ParamList(Vec::new())
        };

        Self {
            export,
            params,
            body: Block::from(inner.next().unwrap()),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for TopLevelKind {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::import => TopLevelKind::Include(Path::from(inner.next().unwrap())),

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

                TopLevelKind::FunctionDecl {
                    export,
                    name,
                    params,
                    return_type,
                    body,
                }
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

                TopLevelKind::StructDecl {
                    export,
                    name,
                    fields,
                }
            }

            Rule::class_decl => TopLevelKind::ClassDecl {
                export: if let Some(first) = inner.peek() {
                    if first.as_rule() == Rule::export {
                        inner.next();
                        true
                    } else {
                        false
                    }
                } else {
                    false
                },
                name: inner.next().unwrap().as_str().to_string(),
                fields: inner
                    .next()
                    .unwrap()
                    .into_inner()
                    .map(VarDeclStmt::from)
                    .collect(),
                constructor: ClassConstructor::from(inner.next().unwrap()),
            },

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
                let mut inner = inner.skip(2);

                Statement::If {
                    initial: pair.into(),
                    else_if: inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .map(StatementBranch::from)
                        .collect(),
                    else_branch: inner.next().map(Statement::from).map(Box::new),
                }
            }

            Rule::while_stmt => Statement::While(pair.into()),

            Rule::c_for_stmt => Statement::CStyleFor {
                init: Box::new(Statement::from(inner.next().unwrap())),
                condition: inner.next().unwrap().into(),
                update: Box::new(Statement::from(inner.next().unwrap())),
                body: Box::new(Statement::from(inner.next().unwrap())),
            },

            Rule::for_stmt => Statement::For {
                pattern: inner.next().unwrap().as_str().to_string(),
                iterator: inner.next().unwrap().into(),
                body: Box::new(Statement::from(inner.next().unwrap())),
            },

            Rule::assign_statement => Statement::VarAssign(VarAssignStmt {
                target: Expression::from(inner.next().unwrap()),
                value: Expression::from(inner.next().unwrap()),
            }),

            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for Literal {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::primary => Self::from(inner.next().unwrap()),
            Rule::integer => Literal::Int(pair.as_str().parse::<i64>().unwrap()),
            Rule::float => Literal::Float(pair.as_str().parse::<f64>().unwrap()),
            Rule::boolean => Literal::Bool(pair.as_str().parse::<bool>().unwrap()),
            Rule::string_lit => Literal::String(inner.as_str().to_string()),
            Rule::tuple => Literal::Tuple(inner.map(Expression::from).collect()),
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
            Rule::static_path => Expression::Path(Path::from(pair)),
            Rule::integer => {
                Expression::Literal(Literal::Int(pair.as_str().parse::<i64>().unwrap()))
            }
            Rule::float => {
                Expression::Literal(Literal::Float(pair.as_str().parse::<f64>().unwrap()))
            }
            Rule::boolean => {
                Expression::Literal(Literal::Bool(pair.as_str().parse::<bool>().unwrap()))
            }
            Rule::string_lit => Expression::Literal(Literal::String(inner.as_str().to_string())),
            Rule::tuple => {
                Expression::Literal(Literal::Tuple(inner.map(Expression::from).collect()))
            }
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

                let type_ = inner.next().and_then(|pair| {
                    if pair.as_str().trim() == "var" {
                        None
                    } else {
                        Some(TypeExpr::from(pair))
                    }
                });
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
