use crate::{
    Rule,
    ast::*,
    ast_expr,
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
            // 1. The top level expressions are now processed via the Pratt Parser
            Rule::expr => {
                static PRATT_PARSER: OnceLock<PrattParser<Rule>> = OnceLock::new();
                let pratt = PRATT_PARSER.get_or_init(|| {
                    use Rule::*;
                    use pest::pratt_parser::{Assoc::*, Op};

                    // Precedence defined from lowest to highest
                    PrattParser::new()
                        .op(Op::infix(or, Left))
                        .op(Op::infix(and, Left))
                        .op(Op::infix(eq, Left)
                            | Op::infix(neq, Left)
                            | Op::infix(lt, Left)
                            | Op::infix(gt, Left)
                            | Op::infix(lte, Left)
                            | Op::infix(gte, Left))
                        .op(Op::infix(add, Left) | Op::infix(sub, Left))
                        .op(Op::infix(mul, Left) | Op::infix(div, Left) | Op::infix(rem, Left))
                });

                pratt
                    .map_primary(|primary_pair| {
                        // Elements handled by map_primary are either sub-expressions or 'term' rules
                        Expression::try_from(primary_pair)
                    })
                    .map_infix(|lhs, op, rhs| {
                        let bin_op = match op.as_rule() {
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

            // 2. The single unit 'term' replaces the old flat 'expr' layout
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

            Rule::primary => pair.into_inner().next().unwrap().try_into(),
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

            Rule::macro_call_px => Ok(Postfix::MacroCall(inner.as_str().to_string())),

            // Note: Rule::binary_px has been completely decoupled from postfix rules
            // as it is now safely managed inside the top-level Pratt execution above.
            _ => AstError::bug_unimplemented(pair),
        }
    }
}
