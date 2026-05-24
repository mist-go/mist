use crate::{
    Rule,
    ast::*,
    ast_ensure, ast_expr,
    error::{AstError, AstResult, IntoErr, collect_recovered},
    parser::listen_rule,
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Block {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.clone().into_inner();

        ast_ensure!(pair, Rule::block => {
            ast_expr!(Block(collect_recovered(inner.next().unwrap().into_inner()), inner.next().map(Spanned::try_from).transpose()))
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for StatementBranch {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.clone().into_inner();

        ast_ensure!(pair, Rule::statement_branch => {
            ast_expr!(StatementBranch {
                condition: inner.next().unwrap().try_into(),
                body: inner.next().unwrap().try_into().map(Box::new),
            })
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Statement {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::statement | Rule::basic_stmt | Rule::control_flow => {
                Statement::try_from(inner.next().unwrap())
            }

            Rule::block => ast_expr!(Statement::Block(pair.try_into())),

            Rule::var_decl_statement => ast_expr!(Statement::VarDecl(pair.try_into())),

            Rule::return_stmt => {
                ast_expr!(Statement::Return(
                    inner.next().map(Expression::try_from).transpose()
                ))
            }

            Rule::break_stmt => Ok(Statement::Break),

            Rule::continue_stmt => Ok(Statement::Continue),

            Rule::if_stmt => {
                ast_expr!(Statement::If {
                    initial: inner.next().unwrap().try_into(),
                    else_if: collect_recovered(inner.next().unwrap().into_inner()),
                    else_branch: inner.next().map(Expression::try_from).transpose(),
                })
            }

            Rule::while_stmt => ast_expr!(Statement::While(inner.next().unwrap().try_into())),

            Rule::loop_stmt => ast_expr!(Statement::Loop(inner.next().unwrap().try_into())),

            Rule::c_for_stmt => ast_expr!(Statement::CStyleFor {
                init: inner.next().unwrap().try_into(),
                condition: inner.next().unwrap().try_into(),
                update: inner.next().unwrap().try_into(),
                body: inner.next().unwrap().try_into(),
            }),

            Rule::for_stmt => ast_expr!(Statement::For {
                mutable: Ok(listen_rule(&mut inner, Rule::mutable)) as AstResult<'_, bool>,
                pattern: inner.next().unwrap().try_into(),
                iterator: inner.next().unwrap().try_into(),
                body: inner.next().unwrap().try_into(),
            }),

            Rule::match_stmt => ast_expr!(Statement::Match(
                inner.next().unwrap().try_into(),
                inner
                    .map(|match_itms| {
                        let mut match_inner = match_itms.into_inner();
                        Ok((
                            collect_recovered(match_inner.next().unwrap().into_inner()).get()?,
                            Expression::try_from(match_inner.next().unwrap()).get()?,
                        ))
                    })
                    .collect::<AstResult<'a, Vec<_>>>(),
            )),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}
