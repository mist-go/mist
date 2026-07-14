use std::path::{Path, PathBuf};

use mist_codegen::RustCodegen;
use mist_parser::rev_mapper::Mapping;
use mist_parser::{MistFmtConfig, parse, parse_module};

pub struct TranspiledFile {
    pub mist_path: PathBuf,
    pub rust_path: PathBuf,
    pub rust_content: String,
    pub mapping: Mapping,
}

#[derive(Debug)]
pub enum TranspileError<'a> {
    Parse(mist_parser::error::ParseError<'a>),
    Semantic(Vec<mist_parser::semantics::SemanticError>),
}

/// Resolve the output `.rs` path for a `.mist` source file.
///
/// Always checks for a `pub module x;` declaration via `parse_module` and
/// uses the declared module name as the filename when present.  Falls back
/// to the literal file stem when parsing fails or the file has no module
/// declaration.
pub fn resolve_rust_path(mist_path: &Path, source: Option<&str>) -> PathBuf {
    let mut rust_path = crate::from_mist_to_rust(mist_path.to_path_buf());
    // Package files (package.mist) must output as <dir>/mod.rs so the Rust
    // module hierarchy resolves correctly.
    if mist_path.file_name().and_then(|n| n.to_str()) == Some("package.mist") {
        rust_path.set_file_name("mod.rs");
    } else if let Some(src) = source {
        // Prioritize the module declaration from the source over the file stem.
        // Every caller re-parses so we always pick up the current declaration.
        if let Ok(Some((_, ref name))) = parse_module(src) {
            if rust_path.file_name().map(|v| v.to_str()).unwrap_or_default() != Some("mod.rs") {
                if let Some(ext) = rust_path.extension().map(|e| e.to_owned()) {
                    let mut new_name = std::ffi::OsString::from(&name.0);
                    new_name.push(".");
                    new_name.push(ext);
                    rust_path.set_file_name(new_name);
                } else {
                    rust_path.set_file_name(&name.0);
                }
            }
        }
    }
    rust_path
}

pub fn transpile_mist<'a>(
    mist_path: &Path,
    source: &'a str,
    extra_mod_decl: &str,
) -> Result<TranspiledFile, TranspileError<'a>> {
    let rust_path = resolve_rust_path(mist_path, Some(source));

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
    let rust_path = resolve_rust_path(mist_path, Some(source));

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

pub fn format_mist(source: &str, _config: MistFmtConfig) -> Result<String, String> {
    // let parsed = parse(source).map_err(|e| format!("parse error: {e:?}"))?;

    // let mut codegen = MistCodegen::new(config);
    // codegen.generate(parsed.mod_attributes);
    // let output = codegen.generate(parsed.items);

    // Ok(output)
    Ok(source.to_string())
}
