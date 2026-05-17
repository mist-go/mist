use crate::{
    Rule,
    ast::*,
    error::{AstError, IntoErr, collect_recovered},
    parser::consume_rule,
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ImplDecl {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::impl_for_decl => Ok(ImplDecl {
                generics: consume_rule(&mut inner, Rule::generics)
                    .map(GenericsDecl::try_from)
                    .transpose()
                    .get()?
                    .unwrap_or_default(),
                trait_: Some(inner.next().unwrap().try_into().get()?),
                target: inner.next().unwrap().try_into().get()?,
                methods: collect_recovered(inner).get()?,
            }),

            Rule::impl_decl => Ok(ImplDecl {
                generics: consume_rule(&mut inner, Rule::generics)
                    .map(GenericsDecl::try_from)
                    .transpose()
                    .get()?
                    .unwrap_or_default(),
                trait_: None,
                target: inner.next().unwrap().try_into().get()?,
                methods: collect_recovered(inner).get()?,
            }),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}
