pub mod decl;
pub mod expr;
pub mod statement;
pub mod types;

use crate::{
    Rule,
    ast::*,
    error::{AstError, GetParseError, collect_recovered},
    parser::consume_rule,
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Identifier {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        Ok(Identifier(pair.as_str().to_string()))
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Path {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::static_path => Ok(Path(collect_recovered(pair.into_inner()).get()?)),
            _ => unimplemented!("{pair:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Literal {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        Ok(match rule {
            Rule::primary => Self::try_from(inner.next().unwrap())?,
            Rule::literal => Self::try_from(inner.next().unwrap())?,
            Rule::integer => Literal::Int(pair.as_str().parse::<i64>().unwrap()),
            Rule::float => Literal::Float(pair.as_str().parse::<f64>().unwrap()),
            Rule::boolean => Literal::Bool(pair.as_str().parse::<bool>().unwrap()),
            Rule::string_lit => Literal::String(inner.as_str().to_string()),
            Rule::tuple => Literal::Tuple(collect_recovered(inner).get()?),
            _ => unimplemented!("{rule:#?}"),
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Pattern {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        Ok(match rule {
            Rule::tuple_pattern => Pattern::Tuple(collect_recovered(pair.into_inner()).get()?),

            Rule::named_tuple_pattern => Pattern::NamedTuple(
                Path::try_from(inner.next().unwrap()).get()?,
                collect_recovered(pair.into_inner()).get()?,
            ),

            Rule::struct_pattern => Pattern::Struct(
                Path::try_from(inner.next().unwrap()).get()?,
                collect_recovered(pair.into_inner()).get()?,
            ),

            Rule::literal => Pattern::Literal(Literal::try_from(pair).get()?),

            Rule::identifier => Pattern::Id(Identifier::try_from(pair).get()?),

            Rule::static_path => Pattern::Path(Path::try_from(pair).get()?),

            _ => unimplemented!("{rule:?}"),
        })
    }
}

impl<'a> TryFrom<&mut pest::iterators::Pairs<'a, Rule>> for Visibility {
    type Error = AstError<'a, Self>;

    fn try_from(pairs: &mut pest::iterators::Pairs<'a, Rule>) -> Result<Self, Self::Error> {
        Ok(consume_rule(pairs, Rule::visibility)
            .map(|pair| -> Result<Visibility, AstError<'a, Self>> {
                if let Some(path) = pair.into_inner().next() {
                    Ok(Visibility::PublicTarget(Path::try_from(path).get()?))
                } else {
                    Ok(Visibility::Public)
                }
            })
            .transpose()?
            .unwrap_or_else(|| Visibility::Private))
    }
}
