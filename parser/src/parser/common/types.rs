use crate::{
    Rule,
    ast::*,
    error::{AstError, IntoErr, collect_recovered},
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
            Rule::tuple_type => Ok(TypeExprKind::Tuple(collect_recovered(inner).get()?)),
            Rule::path_type => {
                let path = Path::try_from(inner.next().unwrap()).get()?;
                let params = collect_recovered(inner).get()?;

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
                inner.next().unwrap().try_into().get()?,
                collect_recovered(inner).get()?,
            )),
            Rule::type_expr_param => Self::try_from(inner.next().unwrap()),
            Rule::lifetime => Ok(TypeExpr(
                TypeExprKind::Lifetime(inner.next().unwrap().try_into().get()?),
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
            Rule::generics => Ok(Generics(collect_recovered(inner).get()?)),
            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Generic {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.clone().into_inner();

        if let Some(pair) = consume_rule(&mut inner, Rule::lifetime) {
            Ok(Generic::Lifetime(
                pair.into_inner().next().unwrap().try_into().get()?,
            ))
        } else {
            Ok(Generic::Type(
                inner.next().unwrap().try_into().get()?,
                collect_recovered(inner).get()?,
            ))
        }
    }
}
