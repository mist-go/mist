use crate::{
    Rule,
    ast::*,
    ast_ensure,
    error::{AstError, collect_recovered, collect_recovered_map},
    parser::consume_rule,
};
use pest::pratt_parser::PrattParser;
use std::sync::OnceLock;

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Expression {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::expr | Rule::expr_no_struct => {
                static PRATT_PARSER: OnceLock<PrattParser<Rule>> = OnceLock::new();
                let pratt = PRATT_PARSER.get_or_init(|| {
                    use pest::pratt_parser::{Assoc::*, Op};

                    PrattParser::new().op(Op::infix(Rule::bin_op, Left))
                });

                pratt
                    .map_primary(|primary_pair| Expression::try_from(primary_pair))
                    .map_infix(|expr, op, rhs| {
                        Ok(Expression::Binary {
                            lhs: expr.map(Box::new)?,
                            op: op.as_str().to_string(),
                            rhs: rhs.map(Box::new)?,
                        })
                    })
                    .parse(inner)
            }

            Rule::term | Rule::term_no_struct => {
                let mut prefix_pairs = Vec::new();
                let mut primary_pair = None;
                let mut postfix_pairs = Vec::new();

                for p in inner {
                    match p.as_rule() {
                        Rule::prefix => prefix_pairs.push(p),
                        Rule::primary => primary_pair = Some(p),
                        Rule::postfix => postfix_pairs.push(p),
                        _ => {}
                    }
                }

                let prefixes = collect_recovered::<Prefix>(prefix_pairs.into_iter())?;
                let exp = Expression::try_from(
                    primary_pair.expect("Term must contain a primary expression"),
                )?;
                let postfixes = collect_recovered::<Postfix>(postfix_pairs.into_iter())?;

                if postfixes.len() > 0 || prefixes.len() > 0 {
                    Ok(Expression::Fix {
                        initial: Box::new(exp),
                        prefixes: prefixes,
                        postfixes: postfixes,
                    })
                } else {
                    Ok(exp)
                }
            }

            Rule::tuple => Ok(Expression::Literal(Literal::Tuple(collect_recovered(
                pair.into_inner(),
            )?))),

            Rule::closure => Ok(Expression::Closure {
                return_type: consume_rule(&mut inner, Rule::type_expr)
                    .map(TypeExpr::try_from)
                    .transpose()?,
                params: collect_recovered(inner.next().unwrap().into_inner())?,
                body: Box::new(Expression::try_from(inner.next().unwrap())?),
            }),

            Rule::array => Ok(Expression::Array(collect_recovered(inner)?)),

            Rule::array_repeat => Ok(Expression::ArrayRepeat(
                Box::new(Expression::try_from(inner.next().unwrap())?),
                Box::new(Expression::try_from(inner.next().unwrap())?),
            )),

            Rule::primary => pair.into_inner().next().unwrap().try_into(),
            Rule::static_path => Ok(Expression::Path(pair.try_into()?)),
            Rule::literal => Ok(Expression::Literal(pair.try_into()?)),
            Rule::expr_path => Ok(Expression::Path(pair.try_into()?)),
            Rule::statement
            | Rule::basic_stmt
            | Rule::control_flow
            | Rule::block
            | Rule::unsafe_block => Ok(Expression::Statement(Box::new(pair.try_into()?))),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Prefix {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        Ok(match pair.as_rule() {
            Rule::prefix => Self::try_from(pair.into_inner().next().unwrap())?,
            Rule::deref_px => Self::Deref,
            Rule::mut_ref_px => Self::RefMut,
            Rule::ref_px => Self::Ref,
            Rule::not_px => Self::Not,
            Rule::neg_px => Self::Neg,

            _ => return AstError::bug_unimplemented(pair),
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Postfix {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::postfix => Postfix::try_from(inner.next().unwrap()),

            Rule::field_px => Ok(Postfix::FieldAccess(
                inner.next().unwrap().try_into()?,
                inner.next().map(Generics::try_from).transpose()?,
            )),

            Rule::tuple_field_px => Ok(Postfix::TupleFieldAccess(
                inner.next().unwrap().as_str().parse().unwrap_or(255_u8),
                inner.next().map(Generics::try_from).transpose()?,
            )),

            Rule::call_px => Ok(Postfix::Call(collect_recovered(inner)?)),

            Rule::struct_px => Ok(Postfix::StructCall(collect_recovered_map(inner, |p| {
                let mut pi = p.into_inner();
                Ok((
                    Identifier::try_from(pi.next().unwrap())?,
                    pi.next().map(Expression::try_from).transpose()?,
                ))
            })?)),

            Rule::index_px => Ok(Postfix::Index(Expression::try_from(inner.next().unwrap())?)),

            Rule::macro_call_paren => Ok(Postfix::MacroCall {
                inner: inner.as_str().to_string(),
                delimiter: MacroDelimiter::Paren,
            }),
            Rule::macro_call_bracket => Ok(Postfix::MacroCall {
                inner: inner.as_str().to_string(),
                delimiter: MacroDelimiter::Bracket,
            }),
            Rule::macro_call_brace => Ok(Postfix::MacroCall {
                inner: inner.as_str().to_string(),
                delimiter: MacroDelimiter::Brace,
            }),

            Rule::as_px => Ok(Postfix::As(inner.next().unwrap().try_into()?)),

            Rule::try_px => Ok(Postfix::Try),

            Rule::increment => Ok(Postfix::Increment),
            Rule::decrement => Ok(Postfix::Decrement),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ExprPath {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        ast_ensure!(pair, Rule::expr_path => {
            Ok(ExprPath(collect_recovered(pair.into_inner())?))
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ExprPathSegment {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.clone().into_inner();

        ast_ensure!(pair, Rule::expr_path_segment => {
            Ok(ExprPathSegment {
                ident: Identifier::try_from(inner.next().unwrap())?,
                generics: inner.next().map(Generics::try_from).transpose()?,
            })
        })
    }
}
