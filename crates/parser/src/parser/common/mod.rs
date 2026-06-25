pub mod decl;
pub mod expr;
pub mod statement;
pub mod types;

use crate::{
    Rule,
    ast::*,
    ast_ensure,
    error::{AstError, collect_recovered, collect_recovered_map},
    parser::{consume_rule, listen_rule},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Identifier {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        ast_ensure!(pair, Rule::identifier => {
            Ok(Identifier(pair.as_str().to_string()))
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Path {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::static_path => Ok(Path(collect_recovered(pair.into_inner())?)),
            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Literal {
    type Error = AstError<'a>;

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
            Rule::tuple => Literal::Tuple(collect_recovered(inner)?),
            _ => return AstError::bug_unimplemented(pair),
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Pattern {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::tuple_pattern => Ok(Pattern::Tuple(collect_recovered_map(inner, |v| {
                Self::try_from(v).map(Box::new)
            })?)),

            Rule::named_tuple_pattern => Ok(Pattern::NamedTuple(
                Path::try_from(inner.next().unwrap())?,
                collect_recovered_map(inner, |v| Self::try_from(v).map(Box::new))?,
            )),

            Rule::struct_pattern => Ok(Pattern::Struct(
                Path::try_from(inner.next().unwrap())?,
                collect_recovered_map(inner, |v| {
                    if v.as_rule() == Rule::etc_pattern {
                        return Ok(None);
                    }

                    let mut inner = v.into_inner();
                    Ok(Some((
                        Identifier::try_from(inner.next().unwrap())?,
                        inner
                            .next()
                            .map(|v| Self::try_from(v).map(Box::new))
                            .transpose()?,
                    )))
                })?,
            )),

            Rule::literal => Ok(Pattern::Literal(pair.try_into()?)),

            Rule::path_pattern => Ok(Pattern::Path(
                listen_rule(&mut inner, Rule::mutable),
                inner.next().unwrap().try_into()?,
            )),

            Rule::etc_pattern => Ok(Pattern::Etc),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<&mut pest::iterators::Pairs<'a, Rule>> for Visibility {
    type Error = AstError<'a>;

    fn try_from(pairs: &mut pest::iterators::Pairs<'a, Rule>) -> Result<Self, Self::Error> {
        Ok(consume_rule(pairs, Rule::visibility)
            .map(|pair| -> Result<Visibility, AstError<'a>> {
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
