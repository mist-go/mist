use std::process::{self};

pub mod builder;
pub mod codegen;
pub mod transpiler;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "run" | "build" | "r" | "b" => {
            let (config, root) = transpiler::build();
            println!("");
            if builder::build(args, config, root) {
                println!("\x1b[32m\nBuild successful\x1b[0m");
            } else {
                println!("\x1b[31m\nBuild failed\x1b[0m");
            }
        }
        "transpile" | "t" => {
            transpiler::build();
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

fn print_usage() {
    println!("Mist - the mist compiler\n");
    println!("󰖟 View our documentation! \x1b[35mhttps://mist.selimaj.dev\x1b[0m");
    println!(" Follow us on github!    \x1b[35mhttps://github.com/mist-go\x1b[0m\n");
    println!("usage:");
    println!(" \x1b[36m mist run\x1b[0m               run the project in the current directory\n");
    println!(
        " \x1b[36m mist build\x1b[0m             build the project in the current directory\n"
    );
    println!(
        " \x1b[36m mist transpile\x1b[0m         transpile the project in the current directory\n"
    );
    println!(" \x1b[36m mist version \x1b[0m          print the compiler version\n");
    println!(" \x1b[36m mist help\x1b[0m              print this message\n");
}
