use crate::{
    Rule,
    ast::*,
    error::{AstError, collect_recovered},
    parser::consume_rule,
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ImplDecl {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::impl_for_decl => Ok(ImplDecl {
                generics: consume_rule(&mut inner, Rule::generics_decl)
                    .map(GenericsDecl::try_from)
                    .transpose()?
                    .unwrap_or_default(),
                trait_: Some(inner.next().unwrap().try_into()?),
                target: inner.next().unwrap().try_into()?,
                methods: collect_recovered(inner)?,
            }),

            Rule::impl_decl => Ok(ImplDecl {
                generics: consume_rule(&mut inner, Rule::generics_decl)
                    .map(GenericsDecl::try_from)
                    .transpose()?
                    .unwrap_or_default(),
                trait_: None,
                target: inner.next().unwrap().try_into()?,
                methods: collect_recovered(inner)?,
            }),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}
