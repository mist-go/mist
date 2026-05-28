use crate::{
    Rule,
    ast::*,
    ast_expr,
    error::{AstError, IntoErr},
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
                    .transpose();

                let name = Pattern::try_from(inner.next().unwrap());

                ast_expr!(VarDecl {
                    type_: type_,
                    name: name,
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
                    type_: TypeExpr::try_from(inner.next().unwrap()),
                    name: Identifier::try_from(inner.next().unwrap()),
                })
            }

            _ => AstError::bug_unimplemented(pair),
        }
    }
}
