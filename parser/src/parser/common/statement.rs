use crate::{
    Rule,
    ast::*,
    ast_expr,
    error::{AstError, AstResult, ErrorCode, IntoErr, collect_recovered},
    parser::listen_rule,
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Block {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        if pair.as_rule() == Rule::block {
            ast_expr!(Block(collect_recovered(pair.into_inner())))
        } else {
            Err(AstError {
                span: pair.as_span(),
                error_code: ErrorCode::InvalidStatement,
                error_message: format!(
                    "BUG: AST requires a block, this isn't a block, it's a {:?}",
                    pair.as_rule()
                ),
                recovered: None,
            })
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Statement {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        Ok(match rule {
            Rule::statement => Statement::try_from(inner.next().unwrap())?,

            Rule::expr_stmt => {
                Statement::Expression(Expression::try_from(inner.next().unwrap()).get()?)
            }

            Rule::block => Statement::Block(pair.try_into().get()?),

            Rule::var_decl_statement => Statement::VarDecl(VarDeclStmt::try_from(pair).get()?),

            Rule::return_stmt => {
                Statement::Return(inner.next().map(Expression::try_from).transpose().get()?)
            }

            Rule::break_stmt => Statement::Break,

            Rule::continue_stmt => Statement::Continue,

            Rule::if_stmt => {
                let mut inner = inner.skip(2);

                Statement::If {
                    initial: StatementBranch::try_from(pair).get()?,
                    else_if: collect_recovered(inner.next().unwrap().into_inner()).get()?,
                    else_branch: inner
                        .next()
                        .map(Statement::try_from)
                        .transpose()?
                        .map(Box::new),
                }
            }

            Rule::while_stmt => Statement::While(pair.try_into().get()?),

            Rule::c_for_stmt => Statement::CStyleFor {
                init: Box::new(inner.next().unwrap().try_into().get()?),
                condition: inner.next().unwrap().try_into().get()?,
                update: Box::new(inner.next().unwrap().try_into().get()?),
                body: Box::new(inner.next().unwrap().try_into().get()?),
            },

            Rule::for_stmt => Statement::For {
                mutable: listen_rule(&mut inner, Rule::mutable),
                pattern: inner.next().unwrap().try_into().get()?,
                iterator: inner.next().unwrap().try_into().get()?,
                body: Box::new(Statement::try_from(inner.next().unwrap())?),
            },

            Rule::assign_statement => Statement::VarAssign(VarAssignStmt {
                target: inner.next().unwrap().try_into().get()?,
                value: inner.next().unwrap().try_into().get()?,
            }),

            Rule::match_stmt => Statement::Match(
                inner.next().unwrap().try_into().get()?,
                inner
                    .map(|match_itms| {
                        let mut match_inner = match_itms.into_inner();
                        Ok((
                            Pattern::try_from(match_inner.next().unwrap()).get()?,
                            Block::try_from(match_inner.next().unwrap()).get()?,
                        ))
                    })
                    .collect::<AstResult<'a, Vec<_>>>()
                    .get()?,
            ),

            Rule::unexpected_statement => {
                return Err(AstError {
                    span: pair.as_span(),
                    error_code: ErrorCode::InvalidStatement,
                    error_message: "Invalid Statement".to_string(),
                    recovered: None,
                });
            }

            _ => unimplemented!("{rule:#?}"),
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for StatementBranch {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.into_inner();

        let condition = inner.next().unwrap().try_into().get()?;
        let body = inner.next().unwrap().try_into().get()?;

        Ok(StatementBranch {
            condition,
            body: Box::new(body),
        })
    }
}
