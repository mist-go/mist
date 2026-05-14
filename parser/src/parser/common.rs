use crate::{
    Rule,
    ast::*,
    error::{ParseError, ParseResult},
    parser::consume_rule,
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Identifier {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        Ok(Identifier(pair.as_str().to_string()))
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Path {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::static_path => Ok(Path(
                pair.into_inner()
                    .map(Identifier::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            )),
            _ => unimplemented!("{pair:#?}"),
        }
    }
}

impl<'a> TryFrom<&mut pest::iterators::Pairs<'a, Rule>> for Visibility {
    type Error = ParseError<'a>;

    fn try_from(pairs: &mut pest::iterators::Pairs<'a, Rule>) -> Result<Self, Self::Error> {
        Ok(consume_rule(pairs, Rule::visibility)
            .map(|pair| -> Result<Visibility, ParseError<'a>> {
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

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Literal {
    type Error = ParseError<'a>;

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
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            ),
            _ => unimplemented!("{rule:#?}"),
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TypeExpr {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::type_expr => Ok(TypeExpr(
                TypeExprKind::try_from(inner.next().unwrap())?,
                inner
                    .map(TypePostfix::try_from)
                    .collect::<ParseResult<'a, _>>()?,
            )),
            Rule::type_expr_param => Self::try_from(inner.next().unwrap()),
            Rule::lifetime => Ok(TypeExpr(
                TypeExprKind::Lifetime(Identifier::try_from(inner.next().unwrap())?),
                Vec::new(),
            )),
            _ => unimplemented!("{rule:#?}"),
        }
    }
}
