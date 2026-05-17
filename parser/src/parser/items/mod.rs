pub mod attribute;
pub mod class;
pub mod enums;
pub mod function;
pub mod impl_decl;

use crate::{
    Rule,
    ast::*,
    ast_expr,
    error::{AstError, AstResult, IntoErr, collect_recovered},
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

        match rule {
            Rule::import => ast_expr!(TopLevelKind::Import(
                Visibility::try_from(&mut inner),
                Path::try_from(inner.next().unwrap()),
            )),

            Rule::function_decl => ast_expr!(TopLevelKind::FunctionDecl(pair.try_into())),

            Rule::struct_decl => ast_expr!(TopLevelKind::StructDecl {
                visibility: Visibility::try_from(&mut inner),

                name: inner.next().unwrap().try_into(),

                generics: consume_rule(&mut inner, Rule::generics_decl)
                    .map(GenericsDecl::try_from)
                    .transpose()
                    .map(|v| v.unwrap_or_default()),

                fields: inner
                    .next()
                    .map(|pair| collect_recovered::<FieldDecl, FieldDecl>(pair.into_inner()))
                    .transpose()
                    .map(|v| v.unwrap_or_default()),
            }),

            Rule::class_decl => ast_expr!(TopLevelKind::ClassDecl {
                visibility: Visibility::try_from(&mut inner),

                name: inner.next().unwrap().try_into(),

                generics: consume_rule(&mut inner, Rule::generics_decl)
                    .map(GenericsDecl::try_from)
                    .transpose()
                    .map(|v| v.unwrap_or_default()),

                fields: collect_recovered(inner.next().unwrap().into_inner()),

                constructor: inner.next().unwrap().try_into(),

                items: collect_recovered(inner),
            }),

            Rule::enum_decl => ast_expr!(TopLevelKind::EnumDecl {
                visibility: Visibility::try_from(&mut inner),

                name: inner.next().unwrap().try_into(),

                generics: consume_rule(&mut inner, Rule::generics_decl)
                    .map(GenericsDecl::try_from)
                    .transpose()
                    .map(|v| v.unwrap_or_default()),

                fields: collect_recovered(inner),
            }),

            Rule::mod_package => ast_expr!(TopLevelKind::Mod(
                Visibility::try_from(&mut inner),
                inner.next().unwrap().try_into(),
            )),

            Rule::impl_for_decl | Rule::impl_decl => {
                ast_expr!(TopLevelKind::ImplDecl(pair.try_into()))
            }

            Rule::trait_decl => ast_expr!(TopLevelKind::TraitDecl {
                visibility: Visibility::try_from(&mut inner),

                name: inner.next().unwrap().try_into(),

                generics: consume_rule(&mut inner, Rule::generics_decl)
                    .map(GenericsDecl::try_from)
                    .transpose()
                    .map(|v| v.unwrap_or_default()),

                requirements: consume_rule(&mut inner, Rule::trait_requirements)
                    .map(|pair| collect_recovered(pair.into_inner()))
                    .transpose()
                    .map(|v| v.unwrap_or_default()),

                items: Ok(Vec::new()) as AstResult<'_, Vec<_>>,
            }),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}
