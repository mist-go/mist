use pest::Parser;
use pest_derive::Parser;

pub mod ast;
pub mod error;

use ast::*;

use crate::error::{ErrorCode, ParseError, ParseResult};

#[derive(Parser)]
#[grammar = "./src/grammar.pest"]
pub struct MistParser;

pub fn parse<'a>(source: &'a str) -> ParseResult<'a, Vec<TopLevel>> {
    let mut pairs = MistParser::parse(Rule::program, source)?;

    let mut statements = vec![];

    for pair in pairs.next().unwrap().into_inner() {
        if pair.as_rule() != Rule::EOI {
            statements.push(TopLevel::try_from(pair)?);
        }
    }

    Ok(statements)
}

impl From<pest::error::Error<Rule>> for ParseError<'_> {
    fn from(value: pest::error::Error<Rule>) -> Self {
        Self::PreAst(value)
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TypeExpr {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::type_expr => Ok(TypeExpr(
                TypeExprKind::try_from(inner.next().unwrap())?,
                inner
                    .map(TypePostfix::try_from)
                    .collect::<ParseResult<'a, _>>()?,
            )),
            Rule::type_expr_param => Self::try_from(inner.next().unwrap()),
            Rule::lifetime => Ok(TypeExpr(
                TypeExprKind::Lifetime(Identifier::try_from(inner.next().unwrap())?),
                Vec::new(),
            )),
            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TypePostfix {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::ref_type => {
                let mutable = listen_rule(&mut inner, Rule::mutable);
                let lifetime = consume_rule(&mut inner, Rule::lifetime)
                    .map(|pair| Identifier::try_from(pair.into_inner().next().unwrap()))
                    .transpose()?;

                Ok(if mutable {
                    if let Some(lifetime) = lifetime {
                        TypePostfix::RefMutLifetime(lifetime)
                    } else {
                        TypePostfix::RefMut
                    }
                } else {
                    if let Some(lifetime) = lifetime {
                        TypePostfix::RefLifetime(lifetime)
                    } else {
                        TypePostfix::Ref
                    }
                })
            }
            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TypeExprKind {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::tuple_type => Ok(TypeExprKind::Tuple(
                inner
                    .map(TypeExpr::try_from)
                    .collect::<ParseResult<'a, _>>()?,
            )),
            Rule::path_type => {
                let path = Path::try_from(inner.next().unwrap())?;
                let params = inner
                    .map(TypeExpr::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?;

                if params.len() == 0 {
                    Ok(TypeExprKind::Path(path))
                } else {
                    Ok(TypeExprKind::PathParams(path, params))
                }
            }
            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Path {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::static_path => Ok(Path(
                pair.into_inner()
                    .map(Identifier::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            )),
            _ => unimplemented!("{pair:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ParamList {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        Ok(ParamList(
            pair.into_inner()
                .map(VarDecl::try_from)
                .collect::<ParseResult<'a, Vec<_>>>()?,
        ))
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Attribute {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::attribute => {
                // unwrap #[ ... ]
                Attribute::try_from(pair.into_inner().next().unwrap())
            }

            Rule::meta => {
                let mut inner = pair.into_inner();

                // first item is always the path
                let path = Path::try_from(inner.next().unwrap())?;

                // check what comes next
                match inner.next() {
                    None => {
                        // #[path]
                        Ok(Attribute::Path(path))
                    }

                    Some(next) => match next.as_rule() {
                        Rule::primary => {
                            // #[path = literal]
                            Ok(Attribute::NameValue {
                                path,
                                value: Literal::try_from(next)?,
                            })
                        }

                        Rule::meta_list => {
                            // #[path(...)]
                            let items = next
                                .into_inner()
                                .map(Attribute::try_from)
                                .collect::<ParseResult<'a, Vec<_>>>()?;

                            Ok(Attribute::List { path, items })
                        }

                        _ => unreachable!("unexpected rule in meta: {:?}", next.as_rule()),
                    },
                }
            }

            Rule::meta_list => {
                // This case usually won't be hit directly,
                // but it's nice to keep it safe if reused
                let items = pair
                    .into_inner()
                    .map(Attribute::try_from)
                    .collect::<Vec<_>>();

                // NOTE: this shouldn't normally construct an Attribute alone
                // but you can panic or wrap depending on your design
                panic!("meta_list should be handled inside meta: {:?}", items);
            }

            _ => unreachable!("unexpected rule: {:?}", pair.as_rule()),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TopLevel {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.into_inner();

        let attributes = inner
            .next()
            .unwrap()
            .into_inner()
            .map(Attribute::try_from)
            .collect::<ParseResult<'a, Vec<_>>>()?;

        Ok(TopLevel(
            inner
                .next()
                .map(TopLevelKind::try_from)
                .unwrap_or(Ok(TopLevelKind::ModAttribute))?,
            attributes,
        ))
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for StatementBranch {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.into_inner();

        let condition = Expression::try_from(inner.next().unwrap())?;
        let body = Statement::try_from(inner.next().unwrap())?;

        Ok(StatementBranch {
            condition,
            body: Box::new(body),
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ClassConstructor {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.into_inner();

        let visibility = Visibility::try_from(&mut inner)?;

        let generics = consume_rule(&mut inner, Rule::generics)
            .map(Generics::try_from)
            .transpose()?
            .unwrap_or_default();

        let params = consume_rule(&mut inner, Rule::param_list)
            .map(ParamList::try_from)
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            visibility,
            generics,
            params,
            body: Block::try_from(inner.next().unwrap())?,
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Generics {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let inner = pair.clone().into_inner();

        match rule {
            Rule::generics => Ok(Generics(
                inner
                    .map(|pair| -> ParseResult<'a, Generic> {
                        let mut inner = pair.into_inner();
                        Ok(
                            if let Some(pair) = consume_rule(&mut inner, Rule::lifetime) {
                                Generic::Lifetime(Identifier::try_from(
                                    pair.into_inner().next().unwrap(),
                                )?)
                            } else {
                                Generic::Type(
                                    Identifier::try_from(inner.next().unwrap())?,
                                    inner
                                        .map(TypeExpr::try_from)
                                        .collect::<ParseResult<'a, Vec<_>>>()?,
                                )
                            },
                        )
                    })
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            )),
            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TopLevelKind {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        Ok(match rule {
            Rule::import => TopLevelKind::Import(
                Visibility::try_from(&mut inner)?,
                Path::try_from(inner.next().unwrap())?,
            ),

            Rule::function_decl => TopLevelKind::FunctionDecl(FunctionDecl::try_from(pair)?),

            Rule::struct_decl => TopLevelKind::StructDecl {
                visibility: Visibility::try_from(&mut inner)?,
                name: Identifier::try_from(inner.next().unwrap())?,
                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()?
                    .unwrap_or_default(),
                fields: inner
                    .next()
                    .map(|pair| {
                        pair.into_inner()
                            .map(FieldDecl::try_from)
                            .collect::<ParseResult<'a, Vec<_>>>()
                    })
                    .transpose()?
                    .unwrap_or_default(),
            },

            Rule::class_decl => TopLevelKind::ClassDecl {
                visibility: Visibility::try_from(&mut inner)?,
                name: Identifier::try_from(inner.next().unwrap())?,
                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()?
                    .unwrap_or_default(),
                fields: inner
                    .next()
                    .unwrap()
                    .into_inner()
                    .map(FieldDeclStmt::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
                constructor: ClassConstructor::try_from(inner.next().unwrap())?,
                items: inner
                    .into_iter()
                    .map(ClassItem::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            },

            Rule::enum_decl => TopLevelKind::EnumDecl {
                visibility: Visibility::try_from(&mut inner)?,
                name: Identifier::try_from(inner.next().unwrap())?,
                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()?
                    .unwrap_or_default(),
                fields: inner
                    .map(EnumItem::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            },

            Rule::mod_package => TopLevelKind::Mod(
                Visibility::try_from(&mut inner)?,
                Identifier::try_from(inner.next().unwrap())?,
            ),

            Rule::impl_for_decl | Rule::impl_decl => {
                TopLevelKind::ImplDecl(ImplDecl::try_from(pair)?)
            }

            Rule::trait_decl => TopLevelKind::TraitDecl {
                visibility: Visibility::try_from(&mut inner)?,
                name: Identifier::try_from(inner.next().unwrap())?,
                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()?
                    .unwrap_or_default(),
                requirements: consume_rule(&mut inner, Rule::trait_requirements)
                    .map(|pair| {
                        pair.into_inner()
                            .map(TypeExpr::try_from)
                            .collect::<ParseResult<'a, Vec<_>>>()
                    })
                    .transpose()?
                    .unwrap_or_default(),
                items: inner
                    .map(FunctionDecl::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            },

            _ => unimplemented!("{rule:#?}"),
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ClassItem {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();

        match rule {
            Rule::impl_decl | Rule::impl_for_decl => {
                Ok(ClassItem::ImplDecl(ImplDecl::try_from(pair)?))
            }

            Rule::method => Ok(ClassItem::Method(FunctionDecl::try_from(pair)?)),

            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ImplDecl {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::impl_for_decl => Ok(ImplDecl {
                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()?
                    .unwrap_or_default(),
                trait_: Some(TypeExpr::try_from(inner.next().unwrap())?),
                target: TypeExpr::try_from(inner.next().unwrap())?,
                methods: inner
                    .map(FunctionDecl::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            }),

            Rule::impl_decl => Ok(ImplDecl {
                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()?
                    .unwrap_or_default(),
                trait_: None,
                target: TypeExpr::try_from(inner.next().unwrap())?,
                methods: inner
                    .map(FunctionDecl::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            }),

            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for EnumItem {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::enum_named => Ok(EnumItem::Named(Identifier::try_from(
                inner.next().unwrap(),
            )?)),

            Rule::enum_tuple => Ok(EnumItem::Tuple(
                Identifier::try_from(inner.next().unwrap())?,
                inner
                    .next()
                    .unwrap()
                    .into_inner()
                    .map(TypeExpr::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            )),

            Rule::enum_struct => Ok(EnumItem::Struct(
                Identifier::try_from(inner.next().unwrap())?,
                inner
                    .next()
                    .map(|pair| {
                        pair.into_inner()
                            .map(FieldDecl::try_from)
                            .collect::<ParseResult<'a, Vec<_>>>()
                    })
                    .transpose()?
                    .unwrap_or_default(),
            )),

            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Block {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let statements = pair
            .into_inner()
            .flat_map(|pair| {
                if pair.as_rule() == Rule::statement_list {
                    pair.into_inner().map(Statement::try_from).collect()
                } else {
                    vec![Statement::try_from(pair)]
                }
            })
            .collect::<ParseResult<'a, Vec<_>>>()?;

        Ok(Block(statements))
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Statement {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        Ok(match rule {
            Rule::statement => Statement::try_from(inner.next().unwrap())?,

            Rule::expr_stmt => Statement::Expression(Expression::try_from(inner.next().unwrap())?),

            Rule::block => Statement::Block(Block::try_from(inner.next().unwrap())?),

            Rule::var_decl_statement => Statement::VarDecl(VarDeclStmt::try_from(pair)?),

            Rule::return_stmt => {
                Statement::Return(inner.next().map(Expression::try_from).transpose()?)
            }

            Rule::break_stmt => Statement::Break,

            Rule::continue_stmt => Statement::Continue,

            Rule::if_stmt => {
                let mut inner = inner.skip(2);

                Statement::If {
                    initial: StatementBranch::try_from(pair)?,
                    else_if: inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .map(StatementBranch::try_from)
                        .collect::<ParseResult<'a, Vec<_>>>()?,
                    else_branch: inner
                        .next()
                        .map(Statement::try_from)
                        .transpose()?
                        .map(Box::new),
                }
            }

            Rule::while_stmt => Statement::While(pair.try_into()?),

            Rule::c_for_stmt => Statement::CStyleFor {
                init: Box::new(Statement::try_from(inner.next().unwrap())?),
                condition: inner.next().unwrap().try_into()?,
                update: Box::new(Statement::try_from(inner.next().unwrap())?),
                body: Box::new(Statement::try_from(inner.next().unwrap())?),
            },

            Rule::for_stmt => Statement::For {
                mutable: listen_rule(&mut inner, Rule::mutable),
                pattern: Pattern::try_from(inner.next().unwrap())?,
                iterator: inner.next().unwrap().try_into()?,
                body: Box::new(Statement::try_from(inner.next().unwrap())?),
            },

            Rule::assign_statement => Statement::VarAssign(VarAssignStmt {
                target: Expression::try_from(inner.next().unwrap())?,
                value: Expression::try_from(inner.next().unwrap())?,
            }),

            Rule::match_stmt => Statement::Match(
                Expression::try_from(inner.next().unwrap())?,
                inner
                    .map(|match_itms| {
                        let mut match_inner = match_itms.into_inner();
                        Ok((
                            Pattern::try_from(match_inner.next().unwrap())?,
                            Block::try_from(match_inner.next().unwrap())?,
                        ))
                    })
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            ),

            Rule::unexpected_statement => {
                return Err(ParseError::Ast {
                    span: pair.as_span(),
                    error_code: ErrorCode::InvalidStatement,
                    error_message: "Invalid Statement".to_string(),
                });
            }

            _ => unimplemented!("{rule:#?}"),
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Literal {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        Ok(match rule {
            Rule::primary => Self::try_from(inner.next().unwrap())?,
            Rule::literal => Self::try_from(inner.next().unwrap())?,
            Rule::integer => Literal::Int(pair.as_str().parse::<i64>().unwrap()),
            Rule::float => Literal::Float(pair.as_str().parse::<f64>().unwrap()),
            Rule::boolean => Literal::Bool(pair.as_str().parse::<bool>().unwrap()),
            Rule::string_lit => Literal::String(inner.as_str().to_string()),
            Rule::tuple => Literal::Tuple(
                inner
                    .map(Expression::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            ),
            _ => unimplemented!("{rule:#?}"),
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Expression {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::expr => {
                let prefixes: Vec<Prefix> = inner
                    .next()
                    .map(|p| {
                        p.into_inner()
                            .into_iter()
                            .map(Prefix::try_from)
                            .collect::<ParseResult<'a, Vec<_>>>()
                    })
                    .transpose()?
                    .unwrap_or_default();

                let exp = Expression::try_from(inner.next().unwrap())?;

                if inner.len() > 0 || prefixes.len() > 0 {
                    Ok(Expression::Fix {
                        initial: Box::new(exp),
                        prefixes,
                        postfixes: inner
                            .map(|p| Postfix::try_from(p))
                            .collect::<ParseResult<'a, Vec<_>>>()?,
                    })
                } else {
                    Ok(exp)
                }
            }
            Rule::primary => Expression::try_from(inner.next().unwrap()),
            Rule::static_path => Ok(Expression::Path(Path::try_from(pair)?)),
            Rule::literal => Ok(Expression::Literal(Literal::try_from(pair)?)),
            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Prefix {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        Ok(match pair.as_rule() {
            Rule::prefix => Self::try_from(pair.into_inner().next().unwrap())?,
            Rule::deref_px => Self::Deref,
            Rule::mut_ref_px => Self::RefMut,
            Rule::ref_px => Self::Ref,
            Rule::new_px => Self::New,
            Rule::not_px => Self::Not,
            _ => unimplemented!("{pair:#?}"),
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Postfix {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        Ok(match rule {
            Rule::postfix => Postfix::try_from(inner.next().unwrap())?,

            Rule::field_px => Postfix::FieldAccess(Identifier::try_from(inner.next().unwrap())?),

            Rule::call_px => Postfix::Call(
                inner
                    .map(Expression::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            ),

            Rule::struct_px => Postfix::StructCall(
                inner
                    .map(|p| {
                        let mut pi = p.into_inner();
                        Ok((
                            Identifier::try_from(pi.next().unwrap())?,
                            Expression::try_from(pi.next().unwrap())?,
                        ))
                    })
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            ),

            Rule::index_px => Postfix::Index(Expression::try_from(inner.next().unwrap())?),

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
                    "&&" => BinaryOp::And,
                    "||" => BinaryOp::Or,

                    _ => {
                        unimplemented!("Binary operator not implemented yet: {}", op_pair.as_str())
                    }
                };
                Postfix::Binary(op, Expression::try_from(inner.next().unwrap())?)
            }

            Rule::macro_call_px => Postfix::MacroCall(inner.as_str().to_string()),

            _ => unimplemented!("{rule:#?}"),
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for VarDeclStmt {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::var_decl_statement => {
                let mut inner = pair.into_inner();

                let decl = VarDecl::try_from(inner.next().unwrap())?;

                let init = inner.next().map(Expression::try_from).transpose()?;

                Ok(VarDeclStmt { decl, init })
            }

            _ => unimplemented!(),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FieldDeclStmt {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::class_field => {
                let mut inner = pair.into_inner();

                let decl = FieldDecl::try_from(inner.next().unwrap())?;

                let init = inner.next().map(Expression::try_from).transpose()?;

                Ok(FieldDeclStmt { decl, init })
            }

            _ => unimplemented!(),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Pattern {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        Ok(match rule {
            Rule::tuple_pattern => Pattern::Tuple(
                inner
                    .map(Identifier::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            ),

            Rule::named_tuple_pattern => Pattern::NamedTuple(
                Path::try_from(inner.next().unwrap())?,
                inner
                    .map(Identifier::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            ),

            Rule::struct_pattern => Pattern::Struct(
                Path::try_from(inner.next().unwrap())?,
                inner
                    .map(Identifier::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            ),

            Rule::literal => Pattern::Literal(Literal::try_from(pair)?),

            Rule::identifier => Pattern::Id(Identifier::try_from(pair)?),

            Rule::static_path => Pattern::Path(Path::try_from(pair)?),

            _ => unimplemented!("{rule:?}"),
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for VarDecl {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::var_decl => {
                let mut inner = pair.into_inner();

                let type_ = inner
                    .next()
                    .and_then(|pair| {
                        if pair.as_str().trim() == "var" {
                            None
                        } else {
                            Some(TypeExpr::try_from(pair))
                        }
                    })
                    .transpose()?;

                let mutable = listen_rule(&mut inner, Rule::mutable);

                let name = Pattern::try_from(inner.next().unwrap())?;

                Ok(VarDecl {
                    mutable,
                    name,
                    type_,
                })
            }

            _ => unimplemented!("{:?}", pair.as_rule()),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FieldDecl {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::field => {
                let mut inner = pair.into_inner();

                let visibility = Visibility::try_from(&mut inner)?;
                let type_ = TypeExpr::try_from(inner.next().unwrap())?;
                let name = Identifier::try_from(inner.next().unwrap())?;

                Ok(FieldDecl {
                    visibility,
                    type_,
                    name,
                })
            }

            _ => unimplemented!("{:?}", pair.as_rule()),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FunctionDecl {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.into_inner();
        let visibility = Visibility::try_from(&mut inner)?;
        let return_type = TypeExpr::try_from(inner.next().unwrap())?;
        let name = Identifier::try_from(inner.next().unwrap())?;
        let generics = consume_rule(&mut inner, Rule::generics)
            .map(Generics::try_from)
            .transpose()?
            .unwrap_or_default();

        let self_param = consume_rule(&mut inner, Rule::self_param).map(|param| {
            let mut param_inner = param.into_inner();
            let name = Pattern::Id(Identifier(String::from("self")));

            let mutable = listen_rule(&mut param_inner, Rule::mutable);

            let is_ref = listen_rule(&mut param_inner, Rule::deref_px);

            VarDecl {
                mutable: mutable && !is_ref,
                name: name.clone(),
                type_: Some(TypeExpr(
                    TypeExprKind::Path(Path(vec![Identifier("Self".to_string())])),
                    if is_ref {
                        vec![if mutable {
                            TypePostfix::RefMut
                        } else {
                            TypePostfix::Ref
                        }]
                    } else {
                        Vec::new()
                    },
                )),
            }
        });

        let params = consume_rule(&mut inner, Rule::param_list)
            .map({
                let self_param = self_param.clone();
                |params_pair| -> ParseResult<'a, ParamList> {
                    let mut params = ParamList::try_from(params_pair)?;
                    if let Some(x) = self_param {
                        params.0.insert(0, x);
                    }
                    Ok(params)
                }
            })
            .transpose()?
            .unwrap_or_else(|| ParamList(self_param.into_iter().collect()));

        let body = inner.next().map(Block::try_from).transpose()?;

        Ok(Self {
            visibility,
            name,
            generics,
            params,
            return_type,
            body,
        })
    }
}
impl<'a> TryFrom<&mut pest::iterators::Pairs<'a, Rule>> for Visibility {
    type Error = ParseError<'a>;

    fn try_from(pairs: &mut pest::iterators::Pairs<'a, Rule>) -> Result<Self, Self::Error> {
        Ok(consume_rule(pairs, Rule::visibility)
            .map(|pair| -> Result<Visibility, ParseError<'a>> {
                if let Some(path) = pair.into_inner().next() {
                    Ok(Visibility::PublicTarget(Path::try_from(path)?))
                } else {
                    Ok(Visibility::Public)
                }
            })
            .transpose()?
            .unwrap_or_else(|| Visibility::Private))
    }
}

pub fn listen_rule(pairs: &mut pest::iterators::Pairs<'_, Rule>, rule: Rule) -> bool {
    let consumed = pairs
        .peek()
        .map(|p| p.as_rule() == rule)
        .unwrap_or_default();

    if consumed {
        pairs.next();
    }

    consumed
}

pub fn consume_rule<'a>(
    pairs: &mut pest::iterators::Pairs<'a, Rule>,
    rule: Rule,
) -> Option<pest::iterators::Pair<'a, Rule>> {
    let consumed = pairs
        .peek()
        .map(|p| p.as_rule() == rule)
        .unwrap_or_default();

    if consumed { pairs.next() } else { None }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Identifier {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        Ok(Identifier(pair.as_str().to_string()))
    }
}
