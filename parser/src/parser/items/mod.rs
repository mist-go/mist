pub mod attribute;
pub mod class;
pub mod enums;
pub mod function;
pub mod impl_decl;

use crate::{
    Rule,
    ast::*,
    error::{ParseError, ParseResult},
    parser::consume_rule,
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TopLevel {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let mut inner = pair.into_inner();

        let attributes = inner
            .next()
            .unwrap()
            .into_inner()
            .map(Attribute::try_from)
            .collect::<ParseResult<'a, Vec<_>>>()?;

        Ok(TopLevel(
            inner
                .next()
                .map(TopLevelKind::try_from)
                .unwrap_or(Ok(TopLevelKind::ModAttribute))?,
            attributes,
        ))
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for TopLevelKind {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        let rule = pair.as_rule();
        let mut inner = pair.clone().into_inner();

        Ok(match rule {
            Rule::import => TopLevelKind::Import(
                Visibility::try_from(&mut inner)?,
                Path::try_from(inner.next().unwrap())?,
            ),

            Rule::function_decl => TopLevelKind::FunctionDecl(FunctionDecl::try_from(pair)?),

            Rule::struct_decl => TopLevelKind::StructDecl {
                visibility: Visibility::try_from(&mut inner)?,
                name: Identifier::try_from(inner.next().unwrap())?,
                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()?
                    .unwrap_or_default(),
                fields: inner
                    .next()
                    .map(|pair| {
                        pair.into_inner()
                            .map(FieldDecl::try_from)
                            .collect::<ParseResult<'a, Vec<_>>>()
                    })
                    .transpose()?
                    .unwrap_or_default(),
            },

            Rule::class_decl => TopLevelKind::ClassDecl {
                visibility: Visibility::try_from(&mut inner)?,
                name: Identifier::try_from(inner.next().unwrap())?,
                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()?
                    .unwrap_or_default(),
                fields: inner
                    .next()
                    .unwrap()
                    .into_inner()
                    .map(FieldDeclStmt::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
                constructor: ClassConstructor::try_from(inner.next().unwrap())?,
                items: inner
                    .into_iter()
                    .map(ClassItem::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            },

            Rule::enum_decl => TopLevelKind::EnumDecl {
                visibility: Visibility::try_from(&mut inner)?,
                name: Identifier::try_from(inner.next().unwrap())?,
                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()?
                    .unwrap_or_default(),
                fields: inner
                    .map(EnumItem::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            },

            Rule::mod_package => TopLevelKind::Mod(
                Visibility::try_from(&mut inner)?,
                Identifier::try_from(inner.next().unwrap())?,
            ),

            Rule::impl_for_decl | Rule::impl_decl => {
                TopLevelKind::ImplDecl(ImplDecl::try_from(pair)?)
            }

            Rule::trait_decl => TopLevelKind::TraitDecl {
                visibility: Visibility::try_from(&mut inner)?,
                name: Identifier::try_from(inner.next().unwrap())?,
                generics: consume_rule(&mut inner, Rule::generics)
                    .map(Generics::try_from)
                    .transpose()?
                    .unwrap_or_default(),
                requirements: consume_rule(&mut inner, Rule::trait_requirements)
                    .map(|pair| {
                        pair.into_inner()
                            .map(TypeExpr::try_from)
                            .collect::<ParseResult<'a, Vec<_>>>()
                    })
                    .transpose()?
                    .unwrap_or_default(),
                items: inner
                    .map(FunctionDecl::try_from)
                    .collect::<ParseResult<'a, Vec<_>>>()?,
            },

            _ => unimplemented!("{rule:#?}"),
        })
    }
}
