use crate::{Rule, ast::*, error::ParseError, parser::consume_rule};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ClassConstructor {
    type Error = ParseError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.into_inner();

        let visibility = Visibility::try_from(&mut inner)?;

        let generics = consume_rule(&mut inner, Rule::generics)
            .map(Generics::try_from)
            .transpose()?
            .unwrap_or_default();

        let params = consume_rule(&mut inner, Rule::param_list)
            .map(ParamList::try_from)
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            visibility,
            generics,
            params,
            body: Block::try_from(inner.next().unwrap())?,
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ClassItem {
    type Error = ParseError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();

        match rule {
            Rule::impl_decl | Rule::impl_for_decl => {
                Ok(ClassItem::ImplDecl(ImplDecl::try_from(pair)?))
            }

            Rule::method => Ok(ClassItem::Method(FunctionDecl::try_from(pair)?)),

            _ => unimplemented!("{rule:#?}"),
        }
    }
}
