use crate::{
    Rule,
    ast::*,
    ast_expr,
    error::{AstError, AstResult, GetParseError, collect_recovered},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Expression {
    type Error = AstError<'a, Self>;

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
                            .collect::<AstResult<'a, Vec<_>, _>>()
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
                            .collect::<AstResult<'a, Vec<_>>>()
                            .get()?,
                    })
                } else {
                    Ok(exp)
                }
            }
            Rule::primary => Expression::try_from(inner.next().unwrap()),
            Rule::static_path => Ok(Expression::Path(Path::try_from(pair).get()?)),
            Rule::literal => Ok(Expression::Literal(Literal::try_from(pair).get()?)),
            _ => unimplemented!("{rule:#?}"),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Prefix {
    type Error = AstError<'a, Self>;

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
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        Ok(match rule {
            Rule::postfix => Postfix::try_from(inner.next().unwrap())?,

            Rule::field_px => ast_expr!(inner@Postfix::FieldAccess(:@@)),

            Rule::call_px => ast_expr!(inner@Postfix::Call(:!*)),

            Rule::struct_px => Postfix::StructCall(
                inner
                    .map(|p| {
                        let mut pi = p.into_inner();
                        Ok((
                            Identifier::try_from(pi.next().unwrap()).get()?,
                            Expression::try_from(pi.next().unwrap()).get()?,
                        ))
                    })
                    .collect::<AstResult<'a, Vec<_>>>()
                    .get()?,
            ),

            Rule::index_px => ast_expr!(inner@Postfix::Index(:@@)),

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

                ast_expr!(inner@Postfix::Binary(:&op, :@@))
            }

            Rule::macro_call_px => Postfix::MacroCall(inner.as_str().to_string()),

            _ => unimplemented!("{rule:#?}"),
        })
    }
}
