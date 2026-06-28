use pest::Parser;
use pest_derive::Parser;

pub mod ast;
pub mod error;
pub mod parser;
pub mod rev_mapper;
pub mod semantics;

use ast::*;

use crate::error::ParseError;

#[derive(Parser)]
#[grammar = "./src/grammar.pest"]
pub struct MistParser;

pub struct Program {
    pub mod_attributes: Vec<TopLevel>,
    pub items: Vec<TopLevel>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MistFmtConfig {
    pub allman_bracket_style: bool,
}

pub fn parse<'a>(source: &'a str) -> Result<Program, ParseError<'a>> {
    let mut pairs = MistParser::parse(Rule::program, source)?;

    let mut statements = vec![];

    for pair in pairs.next().unwrap().into_inner() {
        if pair.as_rule() != Rule::EOI {
            statements.push(TopLevel::try_from(pair)?);
        }
    }

    let (mod_attributes, items): (Vec<_>, Vec<_>) = statements
        .into_iter()
        .partition(|item| matches!(item.0.item, TopLevelKind::ModAttribute));

    Ok(Program {
        items,
        mod_attributes,
    })
}

pub fn parse_module<'a>(
    source: &'a str,
) -> Result<Option<(Visibility, Identifier)>, ParseError<'a>> {
    let mut pairs = MistParser::parse(Rule::module_program, source)?;

    if let Some(v) = pairs
        .next()
        .unwrap()
        .into_inner()
        .next()
        .map(TopLevel::try_from)
        .transpose()?
    {
        if let TopLevelKind::DeclareModule(vis, name) = &v.0.item {
            Ok(Some((vis.clone(), name.clone())))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

#[macro_export]
macro_rules! ast_ensure {
    ($pair:expr, $rule:expr $(, $rules:expr)* => $body:block) => {
        if $pair.as_rule() == $rule $(|| $pair.as_rule() == $rules)*
            $body
        else {
            Err(AstError {
                span: $pair.as_span(),
                error_code: crate::error::ErrorCode::AstGenBug,
                error_message: format!("Possible bug: expected {:?}, got {:?}", $rule, $pair.as_rule()),
            })
        }
    };
}
