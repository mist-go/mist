use std::process::{self, Command};

pub mod codegen;
pub mod compiler;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "run" | "build" | "r" | "b" => {
            compiler::build();
            Command::new("cargo").args(&args[1..]).status().unwrap();
        }
        "transpile" | "t" => compiler::build(),
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

fn print_usage() {
    println!("mist - the mist compiler\n");
    println!("usage:");
    println!(" \x1b[36m mist run\x1b[0m               run the project in the current directory\n");
    println!(" \x1b[36m mist build\x1b[0m             build the project in the current directory\n");
    println!(" \x1b[36m mist transpile\x1b[0m         transpile the project in the current directory\n");
    println!(" \x1b[36m mist version \x1b[0m          print the compiler version\n");
    println!(" \x1b[36m mist help\x1b[0m              print this message\n");
}
