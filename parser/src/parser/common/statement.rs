use crate::{
    Rule,
    ast::*,
    error::{ErrorCode, ParseError, ParseResult},
    parser::listen_rule,
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Block {
    type Error = ParseError<'a, Self>;

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
    type Error = ParseError<'a, Self>;

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

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for StatementBranch {
    type Error = ParseError<'a, Self>;

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
