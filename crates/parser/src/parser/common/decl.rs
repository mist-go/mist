use crate::{
    Rule,
    ast::*,
    ast_expr,
    error::{AstError, IntoErr, collect_recovered},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for VarDeclStmt {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::var_decl_statement => {
                let mut inner = pair.into_inner();

                ast_expr!(VarDeclStmt {
                    decl: inner.next().unwrap().try_into(),
                    init: inner.next().map(Expression::try_from).transpose()
                })
            }

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FieldDeclStmt {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::class_field => {
                let mut inner = pair.into_inner();

                ast_expr!(FieldDeclStmt {
                    decl: inner.next().unwrap().try_into(),
                    init: inner.next().map(Expression::try_from).transpose(),
                })
            }

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for VarDecl {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::var_decl | Rule::param => {
                let mut inner = pair.into_inner();

                ast_expr!(VarDecl {
                    name: Pattern::try_from(inner.next().unwrap()),
                    type_: inner.next().map(TypeExpr::try_from).transpose(),
                })
            }

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FieldDecl {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::field => {
                let mut inner = pair.into_inner();

                ast_expr!(FieldDecl {
                    visibility: Visibility::try_from(&mut inner),
                    name: Identifier::try_from(inner.next().unwrap()),
                    type_: TypeExpr::try_from(inner.next().unwrap()),
                })
            }

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ParamList {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        Ok(ParamList(collect_recovered(pair.into_inner()).get()?))
    }
}
