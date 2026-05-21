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
                    use pest::pratt_parser::{Assoc::*, Op};

                    PrattParser::new().op(Op::infix(Rule::bin_op, Left))
                });

                pratt
                    .map_primary(|primary_pair| Expression::try_from(primary_pair))
                    .map_infix(|expr, op, rhs| {
                        ast_expr!(Expression::Binary {
                            lhs: expr.map(Box::new).get_map(Box::new),
                            op: Ok(op.as_str().to_string()) as AstResult<'_, String>,
                            rhs: rhs.map(Box::new).get_map(Box::new),
                        })
                    })
                    .parse(inner)
            }

            Rule::term => {
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

                let prefixes = collect_recovered::<Prefix, Prefix>(prefix_pairs.into_iter());
                let exp = Expression::try_from(
                    primary_pair.expect("Term must contain a primary expression"),
                );
                let postfixes = collect_recovered::<Postfix, Postfix>(postfix_pairs.into_iter());

                if postfixes.len() > 0 || prefixes.len() > 0 {
                    ast_expr!(Expression::Fix {
                        initial: exp.map(Box::new),
                        prefixes: prefixes,
                        postfixes: postfixes,
                    })
                } else {
                    ast_expr!(use exp?, prefixes, postfixes)
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
            Rule::statement | Rule::basic_stmt | Rule::control_flow => ast_expr!(
                Expression::Statement(pair.try_into().get_map(Box::new).map(Box::new))
            ),

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

            Rule::as_px => {
                ast_expr!(Postfix::As(inner.next().unwrap().try_into()))
            }

            Rule::try_px => Ok(Postfix::Try),

            Rule::increment => Ok(Postfix::Increment),
            Rule::decrement => Ok(Postfix::Decrement),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ExprPath {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        ast_ensure!(pair, Rule::expr_path => {
            ast_expr!(ExprPath(collect_recovered(pair.into_inner())))
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
