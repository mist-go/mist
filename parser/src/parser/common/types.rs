use crate::{
    Rule,
    ast::*,
    ast_ensure, ast_expr,
    error::{AstError, GetLength, IntoErr, collect_recovered},
    parser::{consume_rule, listen_rule},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TypePostfix {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::ref_type => {
                let mutable = listen_rule(&mut inner, Rule::mutable);
                let lifetime = consume_rule(&mut inner, Rule::lifetime)
                    .map(|pair| Identifier::try_from(pair.into_inner().next().unwrap()))
                    .transpose()
                    .get()?;

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

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TypeExprKind {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::tuple_type => ast_expr!(TypeExprKind::Tuple(collect_recovered(inner))),
            Rule::path_type => {
                let path = Path::try_from(inner.next().unwrap());
                let params = collect_recovered(inner);

                if params.len() == 0 {
                    ast_expr!(TypeExprKind::Path(path))
                } else {
                    ast_expr!(TypeExprKind::PathParams(path, params))
                }
            }
            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TypeExpr {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::type_expr => ast_expr!(TypeExpr(
                inner.next().unwrap().try_into(),
                collect_recovered(inner),
            )),
            Rule::type_expr_param => Self::try_from(inner.next().unwrap()),
            Rule::lifetime => ast_expr!(TypeExprKind::Lifetime(inner.next().unwrap().try_into()))
                .get_map(TypeExpr::no_px)
                .map(TypeExpr::no_px),

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
