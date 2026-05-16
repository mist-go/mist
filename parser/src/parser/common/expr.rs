use crate::{
    Rule,
    ast::*,
    ast_expr,
    error::{AstError, GetLength, IntoErr, collect_recovered, collect_recovered_map},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Expression {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::expr => {
                let prefixes = inner
                    .next()
                    .map(|p| collect_recovered::<Prefix, Prefix>(p.into_inner()))
                    .unwrap_or_else(|| Ok(Vec::new()));

                let exp = Expression::try_from(inner.next().unwrap());

                if inner.len() > 0 || prefixes.len() > 0 {
                    ast_expr!(Expression::Fix {
                        initial: exp.map(Box::new),
                        prefixes: prefixes,
                        postfixes: collect_recovered(inner),
                    })
                } else {
                    exp
                }
            }

            Rule::primary => inner.next().unwrap().try_into(),
            Rule::static_path => ast_expr!(Expression::Path(pair.try_into())),
            Rule::literal => ast_expr!(Expression::Literal(pair.try_into())),

            _ => AstError::bug_unimplemented(pair),
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

            _ => return AstError::bug_unimplemented(pair),
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Postfix {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::postfix => Postfix::try_from(inner.next().unwrap()),

            Rule::field_px => {
                ast_expr!(Postfix::FieldAccess(inner.next().unwrap().try_into()))
            }

            Rule::call_px => ast_expr!(Postfix::Call(collect_recovered(inner))),

            Rule::struct_px => ast_expr!(Postfix::StructCall(collect_recovered_map(inner, |p| {
                let mut pi = p.into_inner();
                Ok((
                    Identifier::try_from(pi.next().unwrap())?,
                    Expression::try_from(pi.next().unwrap()).get()?,
                ))
            }))),

            Rule::index_px => {
                ast_expr!(Postfix::Index(Expression::try_from(inner.next().unwrap())))
            }

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

                    _ => return AstError::bug_unimplemented(op_pair),
                };

                Ok(Postfix::Binary(op, inner.next().unwrap().try_into().get()?))
            }

            Rule::macro_call_px => Ok(Postfix::MacroCall(inner.as_str().to_string())),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}
