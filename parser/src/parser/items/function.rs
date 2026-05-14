use crate::{
    Rule,
    ast::*,
    error::{ParseError, ParseResult},
    parser::{consume_rule, listen_rule},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FunctionDecl {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.into_inner();
        let visibility = Visibility::try_from(&mut inner)?;
        let return_type = TypeExpr::try_from(inner.next().unwrap())?;
        let name = Identifier::try_from(inner.next().unwrap())?;
        let generics = consume_rule(&mut inner, Rule::generics)
            .map(Generics::try_from)
            .transpose()?
            .unwrap_or_default();

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
                |params_pair| -> ParseResult<'a, ParamList> {
                    let mut params = ParamList::try_from(params_pair)?;
                    if let Some(x) = self_param {
                        params.0.insert(0, x);
                    }
                    Ok(params)
                }
            })
            .transpose()?
            .unwrap_or_else(|| ParamList(self_param.into_iter().collect()));

        let body = inner.next().map(Block::try_from).transpose()?;

        Ok(Self {
            visibility,
            name,
            generics,
            params,
            return_type,
            body,
        })
    }
}
