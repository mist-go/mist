use std::path::{Path, PathBuf};

use mist_codegen::{GetRust, RustCodegen};
use mist_parser::rev_mapper::Mapping;
use mist_parser::{parse, parse_module};

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
    let module_decl = parse_module(source).map_err(|e| format!("parse module error: {e:?}"))?;
    let rust_path = crate::from_mist_to_rust(mist_path.to_path_buf());

    let parsed = parse(source).map_err(|e| format!("parse error: {e:?}"))?;

    for item in &parsed {
        mist_parser::semantics::check_class_semantics(item)
            .map_err(|e| format!("semantic error: {}", e[0].error_message))?;
    }

    let mut codegen = RustCodegen::new(mist_path.to_path_buf());
    let output = codegen.generate(parsed);

    let self_mod_prefix = module_decl
        .as_ref()
        .map(|(vis, name)| format!("{}mod {};\n", vis.get_rust(), name.get_rust()))
        .unwrap_or_default();

    let combined_prefix = format!("{}{}", extra_mod_decl, self_mod_prefix);

    codegen.mapping.shift_rust(combined_prefix.lines().count() as isize, 0);

    Ok(TranspiledFile {
        mist_path: mist_path.to_path_buf(),
        rust_path,
        rust_content: format!("{}{}", combined_prefix, output),
        mapping: codegen.mapping,
    })
}
