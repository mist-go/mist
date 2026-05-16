pub mod attribute;
pub mod class;
pub mod enums;
pub mod function;
pub mod impl_decl;

use crate::{
    Rule,
    ast::*,
    ast_expr,
    error::{AstError, IntoErr, collect_recovered},
    parser::consume_rule,
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TopLevel {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.into_inner();

        let attributes = collect_recovered(inner.next().unwrap().into_inner());

        ast_expr!(TopLevel(
            inner
                .next()
                .map(TopLevelKind::try_from)
                .unwrap_or(Ok(TopLevelKind::ModAttribute)),
            attributes,
        ))
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TopLevelKind {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        Ok(match rule {
            Rule::import => TopLevelKind::Import(
                Visibility::try_from(&mut inner).get()?,
                Path::try_from(inner.next().unwrap()).get()?,
            ),

            Rule::function_decl => return ast_expr!(TopLevelKind::FunctionDecl(pair.try_into())),

            Rule::struct_decl => TopLevelKind::StructDecl {
                visibility: Visibility::try_from(&mut inner).get()?,

                name: inner.next().unwrap().try_into().get()?,

                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()
                    .get()?
                    .unwrap_or_default(),

                fields: inner
                    .next()
                    .map(|pair| collect_recovered::<FieldDecl, FieldDecl>(pair.into_inner()))
                    .transpose()
                    .get()?
                    .unwrap_or_default(),
            },

            Rule::class_decl => TopLevelKind::ClassDecl {
                visibility: Visibility::try_from(&mut inner).get()?,

                name: inner.next().unwrap().try_into().get()?,

                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()
                    .get()?
                    .unwrap_or_default(),

                fields: collect_recovered(inner.next().unwrap().into_inner()).get()?,

                constructor: inner.next().unwrap().try_into().get()?,

                items: collect_recovered(inner).get()?,
            },

            Rule::enum_decl => TopLevelKind::EnumDecl {
                visibility: Visibility::try_from(&mut inner).get()?,

                name: inner.next().unwrap().try_into().get()?,

                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()
                    .get()?
                    .unwrap_or_default(),

                fields: collect_recovered(inner).get()?,
            },

            Rule::mod_package => TopLevelKind::Mod(
                Visibility::try_from(&mut inner).get()?,
                inner.next().unwrap().try_into().get()?,
            ),

            Rule::impl_for_decl | Rule::impl_decl => TopLevelKind::ImplDecl(pair.try_into().get()?),

            Rule::trait_decl => TopLevelKind::TraitDecl {
                visibility: Visibility::try_from(&mut inner).get()?,

                name: inner.next().unwrap().try_into().get()?,

                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()
                    .get()?
                    .unwrap_or_default(),

                requirements: consume_rule(&mut inner, Rule::trait_requirements)
                    .map(|pair| collect_recovered::<TypeExpr, TypeExpr>(pair.into_inner()))
                    .transpose()
                    .get()?
                    .unwrap_or_default(),

                items: collect_recovered(inner).get()?,
            },

            _ => return AstError::bug_unimplemented(pair),
        })
    }
}
