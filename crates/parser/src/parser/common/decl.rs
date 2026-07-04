use crate::{
    Rule,
    ast::*,
    error::{AstError, collect_recovered},
    parser::{consume_rule, listen_rule},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for VarDeclStmt {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::var_decl_statement => {
                let mut inner = pair.into_inner();

                Ok(VarDeclStmt {
                    decl: inner.next().unwrap().try_into()?,
                    init: inner.next().map(Expression::try_from).transpose()?,
                })
            }

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FieldDeclStmt {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::class_field => {
                let mut inner = pair.into_inner();

                Ok(FieldDeclStmt {
                    decl: inner.next().unwrap().try_into()?,
                    init: inner.next().map(Expression::try_from).transpose()?,
                })
            }

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for VarDecl {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::var_decl | Rule::param => {
                let mut inner = pair.into_inner();

                Ok(VarDecl {
                    type_: consume_rule(&mut inner, Rule::type_expr)
                        .map(TypeExpr::try_from)
                        .transpose()?,
                    true_type: listen_rule(&mut inner, Rule::as_kw),
                    name: Pattern::try_from(inner.next().unwrap())?,
                    tuple_names: collect_recovered(inner)?,
                })
            }

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FieldDecl {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::field => {
                let mut inner = pair.into_inner();

                Ok(FieldDecl {
                    visibility: Visibility::try_from(&mut inner)?,
                    type_: TypeExpr::try_from(inner.next().unwrap())?,
                    name: Identifier::try_from(inner.next().unwrap())?,
                })
            }

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ParamList {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        Ok(ParamList(collect_recovered(pair.into_inner())?))
    }
}
