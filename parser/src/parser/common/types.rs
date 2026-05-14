use crate::{
    Rule,
    ast::*,
    error::{AstError, AstResult},
    parser::{consume_rule, listen_rule},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TypePostfix {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::ref_type => {
                let mutable = listen_rule(&mut inner, Rule::mutable);
                let lifetime = consume_rule(&mut inner, Rule::lifetime)
                    .map(|pair| Identifier::try_from(pair.into_inner().next().unwrap()))
                    .transpose()?;

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
            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TypeExprKind {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::tuple_type => Ok(TypeExprKind::Tuple(
                inner
                    .map(TypeExpr::try_from)
                    .collect::<AstResult<'a, _>>()?,
            )),
            Rule::path_type => {
                let path = Path::try_from(inner.next().unwrap())?;
                let params = inner
                    .map(TypeExpr::try_from)
                    .collect::<AstResult<'a, Vec<_>>>()?;

                if params.len() == 0 {
                    Ok(TypeExprKind::Path(path))
                } else {
                    Ok(TypeExprKind::PathParams(path, params))
                }
            }
            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TypeExpr {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::type_expr => Ok(TypeExpr(
                TypeExprKind::try_from(inner.next().unwrap())?,
                inner
                    .map(TypePostfix::try_from)
                    .collect::<AstResult<'a, _>>()?,
            )),
            Rule::type_expr_param => Self::try_from(inner.next().unwrap()),
            Rule::lifetime => Ok(TypeExpr(
                TypeExprKind::Lifetime(Identifier::try_from(inner.next().unwrap())?),
                Vec::new(),
            )),
            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Generics {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let inner = pair.clone().into_inner();

        match rule {
            Rule::generics => Ok(Generics(
                inner
                    .map(|pair| -> AstResult<'a, Generic> {
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
                                        .collect::<AstResult<'a, Vec<_>>>()?,
                                )
                            },
                        )
                    })
                    .collect::<AstResult<'a, Vec<_>>>()?,
            )),
            _ => unimplemented!("{rule:#?}"),
        }
    }
}
