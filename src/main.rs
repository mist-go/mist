use std::process;

pub mod compiler;

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
            compiler::build();
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
    println!("mist - the mist compiler");
    println!();
    println!("usage:");
    println!("  mist build             compile the project in the current directory");
    println!("  mist check <file.ms>   parse and validate without compiling");
    println!("  mist version           print the compiler version");
    println!("  mist help              print this message");
}
