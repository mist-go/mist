use std::{fs, path::PathBuf, process, time::Instant};

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

    let src = root.join(&config.src);

    for entry in fs::read_dir(src).unwrap() {
        if let Ok(entry) = entry {
            let entry_path = root.join(&entry.path());
            let out_dir = root.join(&config.output);
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            let script = if file_name.ends_with(".ms") {
                true
            } else if file_name.ends_with(".mist") {
                false
            } else {
                continue;
            };

            // 3. read entry file
            let source = match fs::read_to_string(&entry_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: failed to read entry file\n  {}", e);
                    process::exit(1);
                }
            };

            let parser_result = if script {
                parser::script_parser::parse(&source).map_err(|e| e.to_string())
            } else {
                parser::parse(&source).map_err(|e| e.to_string())
            };

            let mut ast = match parser_result {
                Ok(ast) => ast,
                Err(e) => {
                    eprintln!("error: parse failed\n{}", e);
                    process::exit(1);
                }
            };

            semantic::walk_ast(semantic::scope::Scope::from_top(&root, &ast), &mut ast);

            let mut gc = crate::codegen::GoCodegen::new();
            let output = gc.generate(&ast);

            if let Err(e) = fs::create_dir_all(&out_dir) {
                eprintln!("error: failed to create build dir\n  {}", e);
                process::exit(1);
            }

            let out_file =
                out_dir.join(file_name.replace(if script { ".ms" } else { ".mist" }, ".go"));

            if let Err(e) = fs::write(&out_file, output) {
                eprintln!("error: failed to write output\n  {}", e);
                process::exit(1);
            }
        }
    }

    let elapsed = start.elapsed();

    println!("build finished in {:.2?}", elapsed);
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

fn load_config(root: &std::path::Path) -> Config {
    let content =
        std::fs::read_to_string(root.join("mist.json")).expect("failed to read mist.json");

    serde_json::from_str(&content).expect("invalid mist.json format")
}
