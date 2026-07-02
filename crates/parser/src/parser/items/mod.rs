pub mod attribute;
pub mod class;
pub mod enums;
pub mod function;
pub mod impl_decl;

use crate::{
    Rule,
    ast::*,
    error::{AstError, collect_recovered},
    parser::consume_rule,
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TopLevel {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.clone().into_inner();

        let attributes = collect_recovered(inner.next().unwrap().into_inner())?;

        Ok(TopLevel(
            inner
                .next()
                .map(Spanned::try_from)
                .unwrap_or_else(move || Ok(Spanned::new_pair(pair, TopLevelKind::ModAttribute)))?,
            attributes,
        ))
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TopLevelKind {
    type Error = AstError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        match rule {
            Rule::import => Ok(TopLevelKind::Import(
                Visibility::try_from(&mut inner)?,
                Path::try_from(inner.next().unwrap())?,
            )),

            Rule::function_decl => Ok(TopLevelKind::FunctionDecl(pair.try_into()?)),

            Rule::struct_decl => Ok(TopLevelKind::StructDecl {
                visibility: Visibility::try_from(&mut inner)?,

                name: inner.next().unwrap().try_into()?,

                generics: consume_rule(&mut inner, Rule::generics_decl)
                    .map(GenericsDecl::try_from)
                    .transpose()
                    .map(|v| v.unwrap_or_default())?,

                fields: inner
                    .next()
                    .map(|pair| collect_recovered(pair.into_inner()))
                    .transpose()
                    .map(|v| v.unwrap_or_default())?,
            }),

            Rule::class_decl => Ok(TopLevelKind::ClassDecl {
                visibility: Visibility::try_from(&mut inner)?,

                name: inner.next().unwrap().try_into()?,

                generics: consume_rule(&mut inner, Rule::generics_decl)
                    .map(GenericsDecl::try_from)
                    .transpose()
                    .map(|v| v.unwrap_or_default())?,

                inherits: consume_rule(&mut inner, Rule::expr_path)
                    .map(ExprPath::try_from)
                    .transpose()?,

                fields: collect_recovered(inner.next().unwrap().into_inner())?,

                constructor: consume_rule(&mut inner, Rule::class_constructor)
                    .map(Spanned::try_from)
                    .transpose()?,

                items: collect_recovered(inner)?,
            }),

            Rule::enum_decl => Ok(TopLevelKind::EnumDecl {
                visibility: Visibility::try_from(&mut inner)?,

                name: inner.next().unwrap().try_into()?,

                generics: consume_rule(&mut inner, Rule::generics_decl)
                    .map(GenericsDecl::try_from)
                    .transpose()
                    .map(|v| v.unwrap_or_default())?,

                fields: collect_recovered(inner)?,
            }),

            Rule::declare_module => Ok(TopLevelKind::DeclareModule(
                Visibility::try_from(&mut inner)?,
                inner.next().unwrap().try_into()?,
            )),

            Rule::impl_for_decl | Rule::impl_decl => Ok(TopLevelKind::ImplDecl(pair.try_into()?)),

            Rule::trait_decl => Ok(TopLevelKind::TraitDecl {
                visibility: Visibility::try_from(&mut inner)?,

                name: inner.next().unwrap().try_into()?,

                generics: consume_rule(&mut inner, Rule::generics_decl)
                    .map(GenericsDecl::try_from)
                    .transpose()
                    .map(|v| v.unwrap_or_default())?,

                requirements: consume_rule(&mut inner, Rule::trait_requirements)
                    .map(|pair| collect_recovered(pair.into_inner()))
                    .transpose()
                    .map(|v| v.unwrap_or_default())?,

                items: collect_recovered(&mut inner)?,
            }),

            Rule::const_decl => Ok(TopLevelKind::ConstDecl(inner.next().unwrap().try_into()?)),
            Rule::static_decl => Ok(TopLevelKind::StaticDecl(inner.next().unwrap().try_into()?)),

            Rule::type_alias => Ok(TopLevelKind::TypeAlias {
                name: inner.next().unwrap().try_into()?,
                generics: consume_rule(&mut inner, Rule::generics_decl)
                    .map(GenericsDecl::try_from)
                    .transpose()?,
                ty: inner.next().unwrap().try_into()?,
            }),

            _ => AstError::bug_unimplemented(pair),
        }
    }
}
