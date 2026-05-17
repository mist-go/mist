use crate::{
    Rule,
    ast::*,
    ast_ensure, ast_expr,
    error::{AstError, AstResult, IntoErr, collect_recovered},
    parser::{consume_rule, listen_rule},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FunctionDecl {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        ast_ensure!(pair, Rule::function_decl, Rule::method, Rule::method_no_body => {
        let mut inner = pair.into_inner();
            let visibility = Visibility::try_from(&mut inner);
            let return_type = TypeExpr::try_from(inner.next().unwrap());
            let name = Identifier::try_from(inner.next().unwrap());

            let generics = consume_rule(&mut inner, Rule::generics_decl)
                .map(GenericsDecl::try_from)
                .transpose()
                .map(|v| v.unwrap_or_default());

            let self_param = consume_rule(&mut inner, Rule::self_param).map(|param| {
                let mut param_inner = param.into_inner();
                let name = Pattern::Id(Identifier(String::from("self")));

                let mutable = listen_rule(&mut param_inner, Rule::mutable);

                let is_ref = listen_rule(&mut param_inner, Rule::deref_px);

                VarDecl {
                    mutable: mutable && !is_ref,
                    name: name.clone(),
                    type_: Some(TypeExpr(
                        TypeExprKind::Path(Path(vec![Identifier("Self".to_string())])),
                        if is_ref {
                            vec![if mutable {
                                TypePostfix::RefMut
                            } else {
                                TypePostfix::Ref
                            }]
                        } else {
                            Vec::new()
                        },
                    )),
                }
            });

            let params = consume_rule(&mut inner, Rule::param_list)
                .map({
                    let self_param = self_param.clone();
                    |params_pair| -> AstResult<'a, ParamList> {
                        let mut params = ParamList::try_from(params_pair)?;
                        if let Some(x) = self_param {
                            params.0.insert(0, x);
                        }
                        Ok(params)
                    }
                })
                .unwrap_or_else(|| Ok(ParamList(self_param.into_iter().collect())));

            let body = inner.next().map(Block::try_from).transpose();

            ast_expr!(Self {
                visibility: visibility,
                return_type: return_type,
                name: name,
                generics: generics,
                params: params,
                body: body,
            })
        })
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for ParamList {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        Ok(ParamList(collect_recovered(pair.into_inner()).get()?))
    }
}
