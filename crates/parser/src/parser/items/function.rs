use crate::{
    Rule,
    ast::*,
    ast_ensure,
    error::{self, AstError, AstResult},
    parser::{consume_rule, consume_rule_map, listen_rule},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FunctionDecl {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        ast_ensure!(pair, Rule::function_decl => {
        let mut inner = pair.into_inner();
            let visibility = Visibility::try_from(&mut inner)?;

            let return_type = consume_rule(&mut inner, Rule::type_expr)
                .map(TypeExpr::try_from)
                .transpose()?;

            let name = Identifier::try_from(inner.next().unwrap())?;

            let generics = consume_rule(&mut inner, Rule::generics_decl)
                .map(GenericsDecl::try_from)
                .transpose()
                .map(|v| v.unwrap_or_default())?;

            let self_param = consume_rule_map(&mut inner, Rule::self_param, |param| {
                let mut param_inner = param.into_inner();

                let is_ref = listen_rule(&mut param_inner, Rule::ref_px);
                let lifetime = consume_rule(&mut param_inner, Rule::lifetime);
                let mutable = listen_rule(&mut param_inner, Rule::mutable);

                Ok((is_ref, lifetime.map(|v| -> AstResult<Identifier> {
                    Identifier::try_from(v.into_inner().next().unwrap())
                }).transpose()?, mutable))
            })?;

            let params = consume_rule(&mut inner, Rule::param_list).map(ParamList::try_from).transpose()?.unwrap_or_default();

            let is_override = consume_rule(&mut inner, Rule::override_kw).map(Override::try_from).transpose()?;

            let body = inner.next().map(Block::try_from).transpose()?;

            Ok(Self {
                visibility: visibility,
                is_override: is_override,
                return_type: return_type,
                name: name,
                generics: generics,
                self_param: self_param,
                params: params,
                body: body,
            })
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Override {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        ast_ensure!(pair, Rule::override_kw => {
            Ok(Override(pair.into_inner().next().map(ExprPath::try_from).transpose()?))
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for Lifetime {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.clone().into_inner();

        match pair.as_rule() {
            Rule::ref_lifetime => inner.next().unwrap().try_into(),
            Rule::lifetime => Ok(Lifetime::Lifetime(inner.next().unwrap().try_into()?)),
            Rule::unsafe_kw => Ok(Lifetime::Unsafe),
            _ => error::AstError::bug_unimplemented(pair),
        }
    }
}
