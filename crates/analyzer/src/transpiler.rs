use std::path::{Path, PathBuf};

use mist_codegen::RustCodegen;
use mist_parser::parse;
use mist_parser::rev_mapper::Mapping;

pub struct TranspiledFile {
    pub mist_path: PathBuf,
    pub rust_path: PathBuf,
    pub rust_content: String,
    pub mapping: Mapping,
}

#[derive(Debug)]
pub enum TranspileError<'a> {
    Parse(mist_parser::error::ParseError<'a, Vec<mist_parser::ast::TopLevel>>),
    Semantic(Vec<mist_parser::semantics::SemanticError>),
}

pub fn transpile_mist<'a>(
    mist_path: &Path,
    source: &'a str,
    extra_mod_decl: &str,
) -> Result<TranspiledFile, TranspileError<'a>> {
    let mut rust_path = crate::from_mist_to_rust(mist_path.to_path_buf());
    // Package files (package.mist) must output as <dir>/mod.rs so the Rust module
    // hierarchy resolves correctly (pub mod <child>; declarations look for sibling
    // .rs files, and the parent module declaration looks for <dir>/mod.rs).
    if mist_path.file_name().and_then(|n| n.to_str()) == Some("package.mist") {
        rust_path.set_file_name("mod.rs");
    }

    let parsed = parse(source).map_err(TranspileError::Parse)?;

    for item in &parsed.items {
        mist_parser::semantics::check_class_semantics(item).map_err(TranspileError::Semantic)?;
    }

    let mut codegen = RustCodegen::new(mist_path.to_path_buf());

    codegen.generate(parsed.mod_attributes);

    codegen.add(extra_mod_decl);

    let output = codegen.generate(parsed.items);

    Ok(TranspiledFile {
        mist_path: mist_path.to_path_buf(),
        rust_path,
        rust_content: output,
        mapping: codegen.mapping,
    })
}

/// No Semantics
pub fn transpile_mist_no_sem(
    mist_path: &Path,
    source: &str,
    extra_mod_decl: &str,
) -> Result<TranspiledFile, String> {
    let mut rust_path = crate::from_mist_to_rust(mist_path.to_path_buf());
    // Package files (package.mist) must output as <dir>/mod.rs so the Rust module
    // hierarchy resolves correctly (pub mod <child>; declarations look for sibling
    // .rs files, and the parent module declaration looks for <dir>/mod.rs).
    if mist_path.file_name().and_then(|n| n.to_str()) == Some("package.mist") {
        rust_path.set_file_name("mod.rs");
    }

    let parsed = parse(source).map_err(|e| format!("parse error: {e:?}"))?;

    let mut codegen = RustCodegen::new(mist_path.to_path_buf());

    codegen.generate(parsed.mod_attributes);

    codegen.add(extra_mod_decl);

    let output = codegen.generate(parsed.items);

    Ok(TranspiledFile {
        mist_path: mist_path.to_path_buf(),
        rust_path,
        rust_content: output,
        mapping: codegen.mapping,
    })
}
