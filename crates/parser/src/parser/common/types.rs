use crate::{
    Rule,
    ast::*,
    ast_ensure,
    error::{AstError, collect_recovered},
    parser::{consume_rule, listen_rule},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TypeExpr {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::generic => Self::try_from(inner.next().unwrap()),
            Rule::type_expr => {
                let mut inner = inner;

                let mut ty = TypeExpr::try_from(inner.next().unwrap())?;

                for ref_pair in inner {
                    let mut ref_inner = ref_pair.clone().into_inner();

                    match ref_pair.as_rule() {
                        Rule::ref_type => {
                            ty = TypeExpr::Ref {
                                mutable: listen_rule(&mut ref_inner, Rule::mutable),
                                lifetime: consume_rule(&mut ref_inner, Rule::lifetime)
                                    .map(|v| v.into_inner().next().map(Identifier::try_from))
                                    .unwrap_or_default()
                                    .transpose()?,
                                ty: Box::new(ty),
                            };
                        }

                        Rule::unsafe_ref_type => {
                            ty = TypeExpr::UnsafePtr {
                                mutable: listen_rule(&mut ref_inner, Rule::mutable),
                                ty: Box::new(ty),
                            };
                        }

                        Rule::fn_type => {
                            
                        }

                        _ => AstError::bug_unimplemented(ref_pair)?,
                    }
                }

                Ok(ty)
            }
            Rule::lifetime => Ok(TypeExpr::Lifetime(inner.next().unwrap().try_into()?)),

            Rule::void_type => Ok(TypeExpr::Tuple(Vec::new())),
            Rule::tuple_type => Ok(TypeExpr::Tuple(collect_recovered(inner)?)),

            Rule::path_type => Ok(TypeExpr::Path(
                Path::try_from(inner.next().unwrap())?,
                inner.next().map(Generics::try_from).transpose()?,
            )),

            Rule::dyn_type => Ok(TypeExpr::Dyn(
                TypeExpr::try_from(inner.next().unwrap()).map(Box::new)?,
            )),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for GenericsDecl {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let inner = pair.clone().into_inner();

        ast_ensure!(pair, Rule::generics_decl => {
            Ok(GenericsDecl(collect_recovered(inner)?))
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for GenericDecl {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.clone().into_inner();

        ast_ensure!(pair, Rule::generic_decl => {
            if let Some(pair) = consume_rule(&mut inner, Rule::lifetime) {
                Ok(GenericDecl::Lifetime(
                    pair.into_inner().next().unwrap().try_into()?,
                ))
            } else {
                Ok(GenericDecl::Type(
                    inner.next().unwrap().try_into()?,
                    collect_recovered(inner)?,
                ))
            }
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Generics {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let inner = pair.clone().into_inner();

        ast_ensure!(pair, Rule::generics => {
            Ok(Generics(collect_recovered(inner)?))
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Generic {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.clone().into_inner();

        ast_ensure!(pair, Rule::generic => {
            if let Some(pair) = consume_rule(&mut inner, Rule::lifetime) {
                Ok(Generic::Lifetime(
                    pair.into_inner().next().unwrap().try_into()?,
                ))
            } else {
                Ok(Generic::Type(
                    inner.next().unwrap().try_into()?
                ))
            }
        })
    }
}
