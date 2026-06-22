use std::path::{Path, PathBuf};

use mist_codegen::RustCodegen;
use mist_parser::rev_mapper::Mapping;
use mist_parser::parse;

pub struct TranspiledFile {
    pub mist_path: PathBuf,
    pub rust_path: PathBuf,
    pub rust_content: String,
    pub mapping: Mapping,
}

pub fn transpile_mist(
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

    for item in &parsed {
        mist_parser::semantics::check_class_semantics(item)
            .map_err(|e| format!("semantic error: {}", e[0].error_message))?;
    }

    let mut codegen = RustCodegen::new(mist_path.to_path_buf());
    let output = codegen.generate(parsed);

    codegen.mapping.shift_rust(extra_mod_decl.lines().count() as isize, 0);

    Ok(TranspiledFile {
        mist_path: mist_path.to_path_buf(),
        rust_path,
        rust_content: format!("{}{}", extra_mod_decl, output),
        mapping: codegen.mapping,
    })
}
