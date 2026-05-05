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
        statements.push(TopLevel::from(pair));
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

impl From<pest::iterators::Pair<'_, Rule>> for Attribute {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        let rule = pair.as_rule();

        // If we start at the top-level #[...], dive into the meta_item
        if rule == Rule::attribute {
            return Attribute::from(pair.into_inner().next().unwrap());
        }

        let mut inner = pair.into_inner();
        // Every style starts with an identifier (the path)
        let path = StaticPath::from(inner.next().expect("Path identifier expected"));

        let kind = match rule {
            Rule::simple_style => MetaItemKind::Word,

            Rule::key_value_style | Rule::pair => {
                let lit = Literal::from(inner.next().unwrap());
                MetaItemKind::NameValue(lit)
            }

            Rule::list_style => {
                // derive(Debug, Clone) -> NestedMetaItem::MetaItem(Attribute { path: "Debug", kind: Word })
                let items = inner
                    .map(|p| {
                        NestedMetaItem::MetaItem(Attribute {
                            path: StaticPath::from(p),
                            kind: MetaItemKind::Word,
                        })
                    })
                    .collect();
                MetaItemKind::List(items)
            }

            Rule::structured_style => {
                // link(name = "readline") -> NestedMetaItem::MetaItem(Attribute { path: "name", kind: NameValue(...) })
                let items = inner
                    .map(|p| NestedMetaItem::MetaItem(Attribute::from(p)))
                    .collect();
                MetaItemKind::List(items)
            }

            _ => unreachable!("Unexpected rule: {:?}", rule),
        };

        Attribute { path, kind }
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for TopLevel {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        let mut inner = pair.into_inner();
        let mut attributes = Vec::new();

        while inner
            .peek()
            .map(|v| v.as_rule() == Rule::attribute)
            .unwrap_or_default()
        {
            attributes.push(Attribute::from(inner.next().unwrap()));
        }

        TopLevel(inner.next().unwrap().into(), attributes)
    }
}

impl From<pest::iterators::Pair<'_, Rule>> for TopLevelKind {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::import => TopLevelKind::Include(StaticPath::from(inner.next().unwrap())),

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

            Rule::EOI => TopLevelKind::EOI,
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

impl From<pest::iterators::Pair<'_, Rule>> for Literal {
    fn from(pair: pest::iterators::Pair<'_, Rule>) -> Self {
        let rule = pair.as_rule();
        let inner = pair.clone().into_inner();

        match rule {
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
            Rule::static_path => Expression::Path(StaticPath::from(pair)),
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
