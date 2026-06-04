use crate::{
    Rule,
    ast::*,
    error::{AstError, IntoErr},
    parser::consume_rule,
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ClassConstructor {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.into_inner();

        Ok(Self {
            visibility: Visibility::try_from(&mut inner).get()?,

            generics: consume_rule(&mut inner, Rule::generics_decl)
                .map(GenericsDecl::try_from)
                .transpose()
                .get()?
                .unwrap_or_default(),

            params: consume_rule(&mut inner, Rule::param_list)
                .map(ParamList::try_from)
                .transpose()
                .get()?
                .unwrap_or_default(),

            body: inner.next().unwrap().try_into().get()?,
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ClassItem {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();

        match rule {
            Rule::impl_decl | Rule::impl_for_decl => {
                Ok(ClassItem::ImplDecl(pair.try_into().get()?))
            }

            Rule::function_decl => Ok(ClassItem::Method(pair.try_into().get()?)),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}
