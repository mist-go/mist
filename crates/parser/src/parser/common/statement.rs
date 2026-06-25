use crate::{
    Rule,
    ast::*,
    ast_ensure,
    error::{AstError, collect_recovered},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Block {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.clone().into_inner();

        ast_ensure!(pair, Rule::block => {
            Ok(Block {
                statements: collect_recovered(inner.next().unwrap().into_inner())?,
                soft_return: inner.next().map(Spanned::try_from).transpose()?,
            })
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for StatementBranch {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.clone().into_inner();

        ast_ensure!(pair, Rule::statement_branch => {
            Ok(StatementBranch {
                condition: inner.next().unwrap().try_into()?,
                body: Box::new(inner.next().unwrap().try_into()?),
            })
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Statement {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::statement | Rule::basic_stmt | Rule::control_flow => {
                Statement::try_from(inner.next().unwrap())
            }

            Rule::unsafe_block => Ok(Statement::UnsafeBlock(inner.next().unwrap().try_into()?)),

            Rule::block => Ok(Statement::Block(pair.try_into()?)),

            Rule::var_decl_statement => Ok(Statement::VarDecl(pair.try_into()?)),

            Rule::return_stmt => Ok(Statement::Return(
                inner.next().map(Expression::try_from).transpose()?,
            )),

            Rule::break_stmt => Ok(Statement::Break),

            Rule::continue_stmt => Ok(Statement::Continue),

            Rule::if_stmt => Ok(Statement::If {
                initial: inner.next().unwrap().try_into()?,
                else_if: collect_recovered(inner.next().unwrap().into_inner())?,
                else_branch: inner.next().map(Block::try_from).transpose()?,
            }),

            Rule::while_stmt => Ok(Statement::While(inner.next().unwrap().try_into()?)),

            Rule::loop_stmt => Ok(Statement::Loop(inner.next().unwrap().try_into()?)),

            Rule::c_for_stmt => Ok(Statement::CStyleFor {
                init: inner.next().unwrap().try_into()?,
                condition: inner.next().unwrap().try_into()?,
                update: inner.next().unwrap().try_into()?,
                body: inner.next().unwrap().try_into()?,
            }),

            Rule::for_stmt => Ok(Statement::For {
                pattern: inner.next().unwrap().try_into()?,
                iterator: inner.next().unwrap().try_into()?,
                body: inner.next().unwrap().try_into()?,
            }),

            Rule::match_stmt => Ok(Statement::Match(
                inner.next().unwrap().try_into()?,
                collect_recovered(inner)?,
            )),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for MatchItem {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut match_inner = pair.into_inner();

        Ok(MatchItem(
            collect_recovered(match_inner.next().unwrap().into_inner())?,
            Expression::try_from(match_inner.next().unwrap())?,
        ))
    }
}
