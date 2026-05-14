pub mod decl;
pub mod expr;
pub mod statement;
pub mod types;

use crate::{
    Rule,
    ast::*,
    error::{AstError, AstResult},
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
            Rule::static_path => Ok(Path(
                pair.into_inner()
                    .map(Identifier::try_from)
                    .collect::<AstResult<'a, Vec<_>>>()?,
            )),
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
            Rule::tuple => Literal::Tuple(
                inner
                    .map(Expression::try_from)
                    .collect::<AstResult<'a, Vec<_>>>()?,
            ),
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
            Rule::tuple_pattern => Pattern::Tuple(
                inner
                    .map(Identifier::try_from)
                    .collect::<AstResult<'a, Vec<_>>>()?,
            ),

            Rule::named_tuple_pattern => Pattern::NamedTuple(
                Path::try_from(inner.next().unwrap())?,
                inner
                    .map(Identifier::try_from)
                    .collect::<AstResult<'a, Vec<_>>>()?,
            ),

            Rule::struct_pattern => Pattern::Struct(
                Path::try_from(inner.next().unwrap())?,
                inner
                    .map(Identifier::try_from)
                    .collect::<AstResult<'a, Vec<_>>>()?,
            ),

            Rule::literal => Pattern::Literal(Literal::try_from(pair)?),

            Rule::identifier => Pattern::Id(Identifier::try_from(pair)?),

            Rule::static_path => Pattern::Path(Path::try_from(pair)?),

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
                    Ok(Visibility::PublicTarget(Path::try_from(path)?))
                } else {
                    Ok(Visibility::Public)
                }
            })
            .transpose()?
            .unwrap_or_else(|| Visibility::Private))
    }
}
