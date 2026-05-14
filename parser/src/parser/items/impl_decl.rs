use crate::{
    Rule,
    ast::*,
    error::{ParseError, ParseResult},
    parser::consume_rule,
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ImplDecl {
    type Error = ParseError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::impl_for_decl => Ok(ImplDecl {
                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()?
                    .unwrap_or_default(),
                trait_: Some(TypeExpr::try_from(inner.next().unwrap())?),
                target: TypeExpr::try_from(inner.next().unwrap())?,
                methods: inner
                    .map(FunctionDecl::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            }),

            Rule::impl_decl => Ok(ImplDecl {
                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()?
                    .unwrap_or_default(),
                trait_: None,
                target: TypeExpr::try_from(inner.next().unwrap())?,
                methods: inner
                    .map(FunctionDecl::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            }),

            _ => unimplemented!("{rule:#?}"),
        }
    }
}
