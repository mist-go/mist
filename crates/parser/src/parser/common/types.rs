use crate::{
    Rule,
    ast::*,
    ast_ensure, ast_expr,
    error::{AstError, IntoErr, collect_recovered},
    parser::{consume_rule, listen_rule},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TypeExpr {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::generic => Self::try_from(inner.next().unwrap()),
            Rule::type_expr => {
                let mut inner = inner;

                let mut ty = TypeExpr::try_from(inner.next().unwrap())?;

                for ref_pair in inner {
                    let mut ref_inner = ref_pair.into_inner();

                    ty = TypeExpr::Ref {
                        lifetime: consume_rule(&mut ref_inner, Rule::ref_lifetime)
                            .map(|v| v.into_inner().next().map(Lifetime::try_from))
                            .unwrap_or_default()
                            .transpose()
                            .get()?,
                        mutable: listen_rule(&mut ref_inner, Rule::mutable),
                        ty: Box::new(ty),
                    };
                }

                Ok(ty)
            }
            Rule::lifetime => ast_expr!(TypeExpr::Lifetime(inner.next().unwrap().try_into())),

            Rule::tuple_type => ast_expr!(TypeExpr::Tuple(collect_recovered(inner))),
            Rule::path_type => {
                ast_expr!(TypeExpr::Path(
                    Path::try_from(inner.next().unwrap()),
                    inner.next().map(Generics::try_from).transpose()
                ))
            }

            Rule::dyn_type => {
                ast_expr!(TypeExpr::Dyn(
                    TypeExpr::try_from(inner.next().unwrap()).map(Box::new),
                ))
            }

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for GenericsDecl {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let inner = pair.clone().into_inner();

        ast_ensure!(pair, Rule::generics_decl => {
            ast_expr!(GenericsDecl(collect_recovered(inner)))
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for GenericDecl {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.clone().into_inner();

        ast_ensure!(pair, Rule::generic_decl => {
            if let Some(pair) = consume_rule(&mut inner, Rule::lifetime) {
                ast_expr!(GenericDecl::Lifetime(
                    pair.into_inner().next().unwrap().try_into(),
                ))
            } else {
                ast_expr!(GenericDecl::Type(
                    inner.next().unwrap().try_into(),
                    collect_recovered(inner),
                ))
            }
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Generics {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let inner = pair.clone().into_inner();

        ast_ensure!(pair, Rule::generics => {
            ast_expr!(Generics(collect_recovered(inner)))
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Generic {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.clone().into_inner();

        ast_ensure!(pair, Rule::generic => {
            if let Some(pair) = consume_rule(&mut inner, Rule::lifetime) {
                ast_expr!(Generic::Lifetime(
                    pair.into_inner().next().unwrap().try_into(),
                ))
            } else {
                ast_expr!(Generic::Type(
                    inner.next().unwrap().try_into()
                ))
            }
        })
    }
}
