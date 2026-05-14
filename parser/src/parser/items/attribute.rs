use crate::{
    Rule,
    ast::*,
    error::{ParseError, ParseResult},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Attribute {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::attribute => {
                // unwrap #[ ... ]
                Attribute::try_from(pair.into_inner().next().unwrap())
            }

            Rule::meta => {
                let mut inner = pair.into_inner();

                // first item is always the path
                let path = Path::try_from(inner.next().unwrap())?;

                // check what comes next
                match inner.next() {
                    None => {
                        // #[path]
                        Ok(Attribute::Path(path))
                    }

                    Some(next) => match next.as_rule() {
                        Rule::primary => {
                            // #[path = literal]
                            Ok(Attribute::NameValue {
                                path,
                                value: Literal::try_from(next)?,
                            })
                        }

                        Rule::meta_list => {
                            // #[path(...)]
                            let items = next
                                .into_inner()
                                .map(Attribute::try_from)
                                .collect::<ParseResult<'a, Vec<_>>>()?;

                            Ok(Attribute::List { path, items })
                        }

                        _ => unreachable!("unexpected rule in meta: {:?}", next.as_rule()),
                    },
                }
            }

            Rule::meta_list => {
                // This case usually won't be hit directly,
                // but it's nice to keep it safe if reused
                let items = pair
                    .into_inner()
                    .map(Attribute::try_from)
                    .collect::<Vec<_>>();

                // NOTE: this shouldn't normally construct an Attribute alone
                // but you can panic or wrap depending on your design
                panic!("meta_list should be handled inside meta: {:?}", items);
            }

            _ => unreachable!("unexpected rule: {:?}", pair.as_rule()),
        }
    }
}
