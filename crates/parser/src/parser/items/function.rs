use crate::{
    Rule,
    ast::*,
    ast_ensure, ast_expr,
    error::{AstError, AstResult, IntoErr},
    parser::{consume_rule, listen_rule},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FunctionDecl {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        ast_ensure!(pair, Rule::function_decl => {
        let mut inner = pair.into_inner();
            let visibility = Visibility::try_from(&mut inner);
            let is_override: AstResult<bool> = Ok(listen_rule(&mut inner, Rule::override_kw));
            let name = Identifier::try_from(inner.next().unwrap());

            let generics = consume_rule(&mut inner, Rule::generics_decl)
                .map(GenericsDecl::try_from)
                .transpose()
                .map(|v| v.unwrap_or_default());

            let self_param = consume_rule(&mut inner, Rule::self_param).map(|param| {
                let mut param_inner = param.into_inner();
                let is_ref = listen_rule(&mut param_inner, Rule::deref_px);
                let lifetime = consume_rule(&mut param_inner, Rule::lifetime);
                let mutable = listen_rule(&mut param_inner, Rule::mutable);
                let name = Pattern::Path(mutable && !is_ref, Path(vec![Identifier(String::from("self"))]));
                let self_ty = TypeExpr::Path(Path(vec![Identifier(String::from("Self"))]), None);

                VarDecl {
                    name: name.clone(),
                    type_: Some(if is_ref {
                        TypeExpr::Ref {
                            lifetime:
                                lifetime.map(|v| Identifier::try_from(v.into_inner().next().unwrap()))
                                    .transpose()
                                    .expect("Failed to get lifetime identifier"),
                            mutable,
                            ty: Box::new(self_ty)
                        }
                    } else {
                        self_ty
                    }),
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

            let return_type = consume_rule(&mut inner, Rule::type_expr)
                .map(TypeExpr::try_from)
                .transpose();

            let body = inner.next().map(Block::try_from).transpose();

            ast_expr!(Self {
                visibility: visibility,
                is_override: is_override,
                return_type: return_type,
                name: name,
                generics: generics,
                params: params,
                body: body,
            })
        })
    }
}
