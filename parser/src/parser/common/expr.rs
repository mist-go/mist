use crate::{
    Rule,
    ast::*,
    ast_ensure, ast_expr,
    error::{AstError, AstResult, GetLength, IntoErr, collect_recovered, collect_recovered_map},
};
use pest::pratt_parser::PrattParser;
use std::sync::OnceLock;

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Expression {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let inner = pair.clone().into_inner();

        match rule {
            Rule::expr => {
                static PRATT_PARSER: OnceLock<PrattParser<Rule>> = OnceLock::new();
                let pratt = PRATT_PARSER.get_or_init(|| {
                    use Rule::*;
                    use pest::pratt_parser::{Assoc::*, Op};

                    PrattParser::new()
                        .op(Op::infix(range_inc, Left) | Op::infix(range_exc, Left))
                        .op(Op::infix(or, Left))
                        .op(Op::infix(and, Left))
                        .op(Op::infix(bitor, Left))
                        .op(Op::infix(bitxor, Left))
                        .op(Op::infix(bitand, Left))
                        .op(Op::infix(eq, Left) | Op::infix(neq, Left))
                        .op(Op::infix(lt, Left)
                            | Op::infix(lte, Left)
                            | Op::infix(gt, Left)
                            | Op::infix(gte, Left))
                        .op(Op::infix(shl, Left) | Op::infix(shr, Left))
                        .op(Op::infix(add, Left) | Op::infix(sub, Left))
                        .op(Op::infix(mul, Left) | Op::infix(div, Left) | Op::infix(rem, Left))
                });

                pratt
                    .map_primary(|primary_pair| Expression::try_from(primary_pair))
                    .map_infix(|lhs, op, rhs| {
                        let bin_op = match op.as_rule() {
                            Rule::shl => BinaryOp::ShiftLeft,
                            Rule::shr => BinaryOp::ShiftRight,
                            Rule::range_inc => BinaryOp::RangeInclusive,
                            Rule::range_exc => BinaryOp::RangeExclusive,
                            Rule::lte => BinaryOp::LessThanOrEqual,
                            Rule::gte => BinaryOp::GreaterThanOrEqual,
                            Rule::eq => BinaryOp::Equal,
                            Rule::neq => BinaryOp::NotEqual,
                            Rule::and => BinaryOp::And,
                            Rule::or => BinaryOp::Or,
                            Rule::add => BinaryOp::Plus,
                            Rule::sub => BinaryOp::Minus,
                            Rule::mul => BinaryOp::Multiply,
                            Rule::div => BinaryOp::Divide,
                            Rule::rem => BinaryOp::Modulo,
                            Rule::lt => BinaryOp::LessThan,
                            Rule::gt => BinaryOp::GreaterThan,
                            Rule::bitand => BinaryOp::BitAnd,
                            Rule::bitor => BinaryOp::BitOr,
                            Rule::bitxor => BinaryOp::BitXor,
                            _ => return AstError::bug_unimplemented(op),
                        };

                        ast_expr!(Expression::Binary {
                            lhs: lhs.map(Box::new),
                            op: Ok(bin_op) as AstResult<'_, BinaryOp>,
                            rhs: rhs.map(Box::new),
                        })
                    })
                    .parse(inner)
            }

            Rule::term => {
                let mut prefix_pairs = Vec::new();
                let mut primary_pair = None;
                let mut postfix_pairs = Vec::new();

                // Sort out flat layout components
                for p in inner {
                    match p.as_rule() {
                        Rule::prefix => prefix_pairs.push(p),
                        Rule::primary => primary_pair = Some(p),
                        Rule::postfix => postfix_pairs.push(p),
                        _ => {}
                    }
                }

                let prefixes = collect_recovered::<Prefix, Prefix>(prefix_pairs.into_iter());
                let exp = Expression::try_from(
                    primary_pair.expect("Term must contain a primary expression"),
                );
                let postfixes = collect_recovered::<Postfix, Postfix>(postfix_pairs.into_iter());

                // Employs your exact original logic using the GetLength trait
                if postfixes.len() > 0 || prefixes.len() > 0 {
                    ast_expr!(Expression::Fix {
                        initial: exp.map(Box::new),
                        prefixes: prefixes,
                        postfixes: postfixes,
                    })
                } else {
                    exp
                }
            }

            Rule::tuple => {
                ast_expr!(Expression::Literal(
                    collect_recovered(pair.into_inner())
                        .map(Literal::Tuple)
                        .get_map(Literal::Tuple)
                ))
            }

            Rule::primary => pair.into_inner().next().unwrap().try_into(),
            Rule::static_path => ast_expr!(Expression::Path(pair.try_into())),
            Rule::literal => ast_expr!(Expression::Literal(pair.try_into())),
            Rule::expr_path => ast_expr!(Expression::Path(pair.try_into())),

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
            Rule::new_px => Self::New(
                pair.into_inner()
                    .next()
                    .map(|v| v.try_into().get())
                    .transpose()?,
            ),
            Rule::not_px => Self::Not,
            Rule::neg_px => Self::Neg,
            Rule::range_start_inc_px => Self::RangeInclusive,
            Rule::range_start_exc_px => Self::RangeExclusive,

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
                ast_expr!(Postfix::FieldAccess(
                    inner.next().unwrap().try_into(),
                    inner.next().map(Generics::try_from).transpose()
                ))
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

            Rule::macro_call_px => Ok(Postfix::MacroCall(inner.as_str().to_string())),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ExprPath {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        ast_ensure!(pair, Rule::expr_path => {
            ast_expr!(ExprPath {
                segments: collect_recovered(pair.into_inner()),
            })
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ExprPathSegment {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.clone().into_inner();

        ast_ensure!(pair, Rule::expr_path_segment => {
            ast_expr!(ExprPathSegment {
                ident: Identifier::try_from(inner.next().unwrap()),
                generics: inner.next().map(Generics::try_from).transpose(),
            })
        })
    }
}
