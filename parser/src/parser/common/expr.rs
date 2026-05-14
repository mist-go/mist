use crate::{
    Rule,
    ast::*,
    error::{GetParseError, ParseError, ParseResult},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Expression {
    type Error = ParseError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::expr => {
                let prefixes: Vec<Prefix> = inner
                    .next()
                    .map(|p| {
                        p.into_inner()
                            .into_iter()
                            .map(Prefix::try_from)
                            .collect::<ParseResult<'a, Vec<_>, _>>()
                    })
                    .transpose()?
                    .unwrap_or_default();

                let exp = Expression::try_from(inner.next().unwrap())?;

                if inner.len() > 0 || prefixes.len() > 0 {
                    Ok(Expression::Fix {
                        initial: Box::new(exp),
                        prefixes,
                        postfixes: inner
                            .map(|p| Postfix::try_from(p))
                            .collect::<ParseResult<'a, Vec<_>>>()?,
                    })
                } else {
                    Ok(exp)
                }
            }
            Rule::primary => Expression::try_from(inner.next().unwrap()),
            Rule::static_path => Ok(Expression::Path(Path::try_from(pair)?)),
            Rule::literal => Ok(Expression::Literal(Literal::try_from(pair)?)),
            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Prefix {
    type Error = ParseError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        Ok(match pair.as_rule() {
            Rule::prefix => Self::try_from(pair.into_inner().next().unwrap())?,
            Rule::deref_px => Self::Deref,
            Rule::mut_ref_px => Self::RefMut,
            Rule::ref_px => Self::Ref,
            Rule::new_px => Self::New,
            Rule::not_px => Self::Not,
            _ => unimplemented!("{pair:#?}"),
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Postfix {
    type Error = ParseError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        Ok(match rule {
            Rule::postfix => Postfix::try_from(inner.next().unwrap())?,

            Rule::field_px => Postfix::FieldAccess(Identifier::try_from(inner.next().unwrap())?),

            Rule::call_px => Postfix::Call(inner.map(Expression::try_from).collect::<ParseResult<
                'a,
                Vec<_>,
                _,
            >>()),

            Rule::struct_px => Postfix::StructCall(
                inner
                    .map(|p| {
                        let mut pi = p.into_inner();
                        Ok((
                            Identifier::try_from(pi.next().unwrap())?,
                            Expression::try_from(pi.next().unwrap())?,
                        ))
                    })
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            ),

            Rule::index_px => Postfix::Index(Expression::try_from(inner.next().unwrap()).get()?),

            Rule::binary_px => {
                let op_pair = inner.next().unwrap();
                let op = match op_pair.as_str() {
                    "+" => BinaryOp::Plus,
                    "-" => BinaryOp::Minus,
                    "*" => BinaryOp::Multiply,
                    "/" => BinaryOp::Divide,
                    "%" => BinaryOp::Modulo,
                    "==" => BinaryOp::Equal,
                    "!=" => BinaryOp::NotEqual,
                    "<" => BinaryOp::LessThan,
                    ">" => BinaryOp::GreaterThan,
                    "<=" => BinaryOp::LessThanOrEqual,
                    ">=" => BinaryOp::GreaterThanOrEqual,
                    "&&" => BinaryOp::And,
                    "||" => BinaryOp::Or,

                    _ => {
                        unimplemented!("Binary operator not implemented yet: {}", op_pair.as_str())
                    }
                };
                Postfix::Binary(op, Expression::try_from(inner.next().unwrap())?)
            }

            Rule::macro_call_px => Postfix::MacroCall(inner.as_str().to_string()),

            _ => unimplemented!("{rule:#?}"),
        })
    }
}
