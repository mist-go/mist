use crate::{
    Rule,
    ast::*,
    error::{AstError, IntoErr, collect_recovered},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for EnumItem {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::enum_named => Ok(EnumItem::Named(inner.next().unwrap().try_into().get()?)),

            Rule::enum_tuple => Ok(EnumItem::Tuple(
                inner.next().unwrap().try_into().get()?,
                collect_recovered(inner.next().unwrap().into_inner()).get()?,
            )),

            Rule::enum_struct => Ok(EnumItem::Struct(
                inner.next().unwrap().try_into().get()?,
                inner
                    .next()
                    .map(|pair| collect_recovered::<FieldDecl, FieldDecl>(pair.into_inner()))
                    .transpose()
                    .get()?
                    .unwrap_or_default(),
            )),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}
