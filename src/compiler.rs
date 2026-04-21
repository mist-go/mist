use std::{fs, path::PathBuf, process, time::Instant};

use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    entry: String,
    out_dir: String,
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

    let entry_path = root.join(&config.entry);
    let out_dir = root.join(&config.out_dir);

    println!("  → entry: {}", entry_path.display());

    // 3. read entry file
    let source = match fs::read_to_string(&entry_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to read entry file\n  {}", e);
            process::exit(1);
        }
    };

    println!("  → parsing...");

    let mut ast = match parser::parse(&source) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("error: parse failed\n{}", e);
            process::exit(1);
        }
    };

    println!("  → type checking...");

    semantic::walk_ast(semantic::scope::Scope::from_top(&ast), &mut ast);

    println!("  → generating Go code...");

    let mut gc = codegen::GoCodegen::new();
    let output = gc.generate(&ast);

    // 4. ensure build dir
    if let Err(e) = fs::create_dir_all(&out_dir) {
        eprintln!("error: failed to create build dir\n  {}", e);
        process::exit(1);
    }

    let out_file = out_dir.join("main.go");

    if let Err(e) = fs::write(&out_file, output) {
        eprintln!("error: failed to write output\n  {}", e);
        process::exit(1);
    }

    let elapsed = start.elapsed();

    println!("  ✓ built {}", out_file.display());
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
