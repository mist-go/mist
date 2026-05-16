use std::{
    fs,
    path::{Path, PathBuf},
    process,
    time::Instant,
};

use mist_parser::error::ParseError;
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    src: String,
    output: String,
}

pub fn build() {
    let start = Instant::now();

    // 1. find project root
    let root = find_project_root().unwrap_or_else(|| {
        panic!("error: could not find project root (mist.json)");
    });

    println!("mistc build ({})", root.display());

    // 2. load config
    let config = load_config(&root);

    let src_dir = root.join(&config.src);
    let out_dir = root.join(&config.output);

    build_dir(&root, &src_dir, &src_dir, &out_dir);

    let elapsed = start.elapsed();

    println!("build finished in {:.2?}", elapsed);
}

fn build_dir(root: &Path, base_src: &Path, current_dir: &Path, out_dir: &Path) {
    let entries = match fs::read_dir(current_dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!(
                "error: failed to read directory {}\n  {}",
                current_dir.display(),
                e
            );

            process::exit(1);
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("error: failed to read directory entry\n  {}", e);

                process::exit(1);
            }
        };

        let path = entry.path();

        // recurse into nested directories
        if path.is_dir() {
            build_dir(root, base_src, &path, out_dir);
            continue;
        }

        // skip non-mist files
        if path.extension().and_then(|e| e.to_str()) != Some("mist") {
            continue;
        }

        let relative = path.strip_prefix(base_src).unwrap();

        let output_path = out_dir.join(relative).with_extension("rs");

        // create parent directories
        if let Some(parent) = output_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!(
                    "error: failed to create output directory {}\n  {}",
                    parent.display(),
                    e
                );

                process::exit(1);
            }
        }

        // read source
        let source = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: failed to read file {}\n  {}", path.display(), e);

                process::exit(1);
            }
        };

        let parser_result = mist_parser::parse(&source).map_err(|e| match e {
            ParseError::Ast(e) => {
                let start_pos = e.span.start_pos().line_col();

                let span = e.span.as_str();

                format!(
                    "\n{}:{}:{}\n \x1b[31mError\x1b[0m: {}\n\t{}{}\t{}",
                    path.as_os_str().display(),
                    start_pos.0,
                    start_pos.1,
                    e.error_message,
                    span,
                    if span.ends_with("\n") { "" } else { "\n" },
                    "^".repeat(span.trim().len()),
                )
            }
            ParseError::PreAst(e) => format!("{e}"),
        });

        let ast = match parser_result {
            Ok(ast) => ast,
            Err(e) => {
                eprintln!("error: parse failed in {}\n{}", path.display(), e);

                process::exit(1);
            }
        };

        // semantic::walk_ast(semantic::scope::Scope::from_top(root, &ast), &mut ast);

        let mut gc = crate::codegen::RustCodegen::new();
        let output = gc.generate(&ast);

        if let Err(e) = fs::write(&output_path, output) {
            eprintln!(
                "error: failed to write output {}\n  {}",
                output_path.display(),
                e
            );

            process::exit(1);
        }
    }
}

pub fn find_project_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;

    loop {
        if dir.join("mist.json").exists() {
            return Some(dir);
        }

        if !dir.pop() {
            return None;
        }
    }
}

fn load_config(root: &Path) -> Config {
    let content = fs::read_to_string(root.join("mist.json")).expect("failed to read mist.json");

    serde_json::from_str(&content).expect("invalid mist.json format")
}
