use crate::{
    Rule,
    ast::*,
    error::{ParseError, ParseResult},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for EnumItem {
    type Error = ParseError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::enum_named => Ok(EnumItem::Named(Identifier::try_from(
                inner.next().unwrap(),
            )?)),

            Rule::enum_tuple => Ok(EnumItem::Tuple(
                Identifier::try_from(inner.next().unwrap())?,
                inner
                    .next()
                    .unwrap()
                    .into_inner()
                    .map(TypeExpr::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            )),

            Rule::enum_struct => Ok(EnumItem::Struct(
                Identifier::try_from(inner.next().unwrap())?,
                inner
                    .next()
                    .map(|pair| {
                        pair.into_inner()
                            .map(FieldDecl::try_from)
                            .collect::<ParseResult<'a, Vec<_>>>()
                    })
                    .transpose()?
                    .unwrap_or_default(),
            )),

            _ => unimplemented!("{rule:#?}"),
        }
    }
}
