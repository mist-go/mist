use std::fs;
use std::path::PathBuf;
use std::process;

use semantic::walk_ast;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "build" => {
            if args.len() < 2 {
                eprintln!("error: expected a file path\n  usage: mist build");
                process::exit(1);
            }
            cmd_build();
        }
        "check" => {
            if args.len() < 3 {
                eprintln!("error: expected a file path\n  usage: mist check <file.ms>");
                process::exit(1);
            }
            cmd_check(&args[2]);
        }
        "parse" => {
            if args.len() < 3 {
                eprintln!("error: expected a file path\n  usage: mist check <file.ms>");
                process::exit(1);
            }
            cmd_parse(&args[2]);
        }
        "version" | "--version" | "-v" => {
            println!("mist {}", env!("CARGO_PKG_VERSION"));
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        unknown => {
            eprintln!("error: unknown command '{}'\n", unknown);
            print_usage();
            process::exit(1);
        }
    }
}

pub fn cmd_build() {
    unimplemented!("build command is not implemented yet");
}

fn cmd_check(path: &str) {
    let source = read_ms_file(path);
    match parser::parse(&source) {
        Ok(mut ast) => {
            println!("parse: ok");

            walk_ast(semantic::scope::Scope::from_top(&ast), &mut ast);

            println!("{:#?}", ast)
        }
        Err(e) => {
            eprintln!("parse error:\n{}", e);
            process::exit(1);
        }
    }
}

fn cmd_parse(path: &str) {
    let source = read_ms_file(path);
    match parser::parse(&source) {
        Ok(mut ast) => {
            walk_ast(semantic::scope::Scope::from_top(&ast), &mut ast);

            fs::write("output.json", serde_json::to_string_pretty(&ast).unwrap()).unwrap_or_else(
                |e| {
                    eprintln!("error: could not write output.json: {}", e);
                    process::exit(1);
                },
            );
        }
        Err(e) => {
            eprintln!("parse error:\n{}", e);
            process::exit(1);
        }
    }
}

fn read_ms_file(path: &str) -> String {
    let pb = PathBuf::from(path);

    if !pb.exists() {
        eprintln!("error: file '{}' not found", path);
        process::exit(1);
    }

    if pb.extension().and_then(|e| e.to_str()) != Some("ms") {
        eprintln!("error: expected a .ms file, got '{}'", path);
        process::exit(1);
    }

    fs::read_to_string(&pb).unwrap_or_else(|e| {
        eprintln!("error: could not read '{}': {}", path, e);
        process::exit(1);
    })
}

fn print_usage() {
    println!("mist - the mist compiler");
    println!();
    println!("usage:");
    println!("  mist build             compile the project in the current directory");
    println!("  mist check <file.ms>   parse and validate without compiling");
    println!("  mist version           print the compiler version");
    println!("  mist help              print this message");
}
