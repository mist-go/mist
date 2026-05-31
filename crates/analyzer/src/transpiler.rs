use std::{
    fs,
    path::{Path, PathBuf},
};

use mist_parser::{ast::TopLevel, error::ParseError};

pub fn build(root: &PathBuf) {
    let src_dir = root.join("src");
    let out_dir = root.join(".mist/src");

    if let Err(e) = build_dir(root, &src_dir, &src_dir, &out_dir) {
        eprintln!("Warning: Build directory run aborted safely: {e}");
    }
}

fn build_dir(
    root: &Path,
    base_src: &Path,
    current_dir: &Path,
    out_dir: &Path,
) -> Result<(), String> {
    let entries = fs::read_dir(current_dir)
        .map_err(|e| format!("failed to read directory {}: {}", current_dir.display(), e))?;

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("Warning: Skipping invalid directory entry: {e}");
                continue; // Skip corrupted entry instead of crashing
            }
        };

        let path = entry.path();

        // Recurse into nested directories
        if path.is_dir() {
            if let Err(e) = build_dir(root, base_src, &path, out_dir) {
                eprintln!("Warning: Nested build directory failed: {e}");
            }
            continue;
        }

        // Safe prefix stripping fallback
        let relative = match path.strip_prefix(base_src) {
            Ok(p) => p,
            Err(_) => {
                eprintln!(
                    "Warning: Path {} is outside base source directory",
                    path.display()
                );
                continue;
            }
        };

        // Handle non-mist files with a cache check
        if path.extension().and_then(|e| e.to_str()) != Some("mist") {
            let dest_path = out_dir.join(relative);

            // Create parent directories for static assets if needed
            if let Some(parent) = dest_path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            if should_skip(&path, &dest_path) {
                continue;
            }

            if let Err(e) = fs::copy(&path, &dest_path) {
                eprintln!(
                    "Warning: Failed to copy non-mist file {}: {}",
                    path.display(),
                    e
                );
            }
            continue;
        }

        let output_path = out_dir.join(relative).with_extension("rs");

        // Cache layer: Skip if the generated .rs file is newer than the .mist source
        if should_skip(&path, &output_path) {
            continue;
        }

        if let Err(e) = transpile_file(&path, &output_path) {
            eprintln!(
                "Warning: Transpilation failed for {}: {}",
                path.display(),
                e
            );
        }
    }

    Ok(())
}

pub fn transpile_file(path: &Path, output_path: &Path) -> Result<(), String> {
    // Create parent directories
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create output directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    // Read source
    let source = fs::read_to_string(path)
        .map_err(|e| format!("failed to read file {}: {}", path.display(), e))?;

    let parser_result = mist_parser::parse(&source).map_err(|e| match e {
        ParseError::Ast(e) => {
            // Using components carefully to prevent out-of-bounds or zero layout crashes
            let start_pos = e.span.start_pos().line_col();
            let span = e.span.as_str();

            format!(
                "\n{}:{}:{}\n Error: {}\n\t{}{}\t{}",
                path.display(),
                start_pos.0,
                start_pos.1,
                e.error_message,
                span,
                if span.ends_with('\n') { "" } else { "\n" },
                "^".repeat(span.trim().len()),
            )
        }
        ParseError::PreAst(e) => format!("{e}"),
    });

    // If parsing fails, return the error string back gracefully so the LSP can show it
    let ast = parser_result.map_err(|e| format!("parse failed in {}:\n{}", path.display(), e))?;

    let mut gc = mist_codegen::RustCodegen::new();
    let output = gc.generate(ast);

    fs::write(output_path, output)
        .map_err(|e| format!("failed to write output {}: {}", output_path.display(), e))?;

    Ok(())
}

pub fn transpile_text<'a>(source: &'a str) -> Result<String, ParseError<'a, Vec<TopLevel>>> {
    let mut gc = mist_codegen::RustCodegen::new();

    Ok(gc.generate(mist_parser::parse(&source)?))
}

fn should_skip(source: &Path, output: &Path) -> bool {
    if let (Ok(src_meta), Ok(out_meta)) = (fs::metadata(source), fs::metadata(output)) {
        if let (Ok(src_time), Ok(out_time)) = (src_meta.modified(), out_meta.modified()) {
            return out_time >= src_time;
        }
    }
    false
}
