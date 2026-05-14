use pest::Parser;
use pest_derive::Parser;

pub mod ast;
pub mod error;
pub mod parser;

use ast::*;

use crate::error::{ErrorCode, ParseError, ParseResult};

use crate::parser::{consume_rule, listen_rule};

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
