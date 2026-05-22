use std::{
    collections::HashMap,
    env, fs,
    path::{MAIN_SEPARATOR, PathBuf},
    process::{Command, Stdio},
};

use cargo_metadata::{CompilerMessage, Message};
use mist_parser::rev_mapper::{RustMap, find_mapping, get_mapping};

#[derive(Debug, Clone)]
pub struct MistDiagnosticMessage {
    pub message: String,
    pub file_path: PathBuf,
    pub file_name: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub enum MistDiagnostic {
    Error(MistDiagnosticMessage),
    Warning(MistDiagnosticMessage),
    Rust(CompilerMessage),
}

pub fn build(mut args: Vec<String>, root: PathBuf) -> Vec<MistDiagnostic> {
    args.insert(1, "--message-format=json".to_string());

    let is_root = root == env::current_dir().expect("Failed getting env");

    let mut command = Command::new("cargo")
        .args(args)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to run cargo");

    let mut reader = std::io::BufReader::new(command.stdout.take().expect("Failed to get reader"));

    let mut diagnostics = Vec::new();

    let mut mapping = HashMap::new();

    let mist_src = format!(".mist{MAIN_SEPARATOR}lsp");

    for message in cargo_metadata::Message::parse_stream(&mut reader) {
        match message {
            Ok(Message::CompilerMessage(msg)) => {
                for span in &msg.message.spans {
                    if span.is_primary {
                        let rust_path = root.join(&span.file_name);

                        let mist_file = span
                            .file_name
                            .replacen(&mist_src, "src", 1)
                            .trim_end_matches(".rs")
                            .to_string()
                            + ".mist";

                        let mist_path = root.join(&mist_file);

                        if !fs::exists(&mist_path).expect("Unable to check if mist file exists") {
                            diagnostics.push(MistDiagnostic::Rust(msg));
                            break;
                        }

                        let map = mapping.entry(rust_path.clone()).or_insert_with(|| {
                            get_mapping(
                                &fs::read_to_string(rust_path)
                                    .expect("Failed to read file for mapping"),
                            )
                        });

                        let mist_span =
                            find_mapping(&map, &RustMap(span.line_end, span.column_start))
                                .expect("Unable to find mapping");

                        let mist_msg = MistDiagnosticMessage {
                            message: span.label.clone().unwrap_or(msg.message.message.clone()),
                            file_name: if is_root {
                                mist_file
                            } else {
                                mist_path.to_string_lossy().to_string()
                            },
                            file_path: mist_path,
                            line: mist_span.1.0,
                            column: mist_span.1.1,
                        };

                        match msg.message.level {
                            cargo_metadata::diagnostic::DiagnosticLevel::Error => {
                                diagnostics.push(MistDiagnostic::Error(mist_msg))
                            }

                            cargo_metadata::diagnostic::DiagnosticLevel::Warning => {
                                diagnostics.push(MistDiagnostic::Warning(mist_msg))
                            }

                            _ => {}
                        }
                    }
                }
            }

            Ok(Message::BuildFinished(_)) => {
                command.wait().unwrap();

                return diagnostics;
            }

            Ok(Message::TextLine(text)) => println!("{text}"),
            _ => {}
        }
    }

    diagnostics
}

// pub fn print_diagnostics(diagnostics: &Vec<MistDiagnostic>) {
//     let mut files = HashMap::new();

//     for diag in diagnostics {
//         match diag {
//             MistDiagnostic::Error(msg) => {
//                 let line = get_line(&mut files, &msg);

//                 println!(
//                     "\n{}:{}:{}\n \x1b[31mError\x1b[0m: {}\n\t{}",
//                     msg.file_name,
//                     msg.line,
//                     msg.column,
//                     msg.message,
//                     line.unwrap_or_default(),
//                 )
//             }

//             MistDiagnostic::Warning(msg) => {
//                 let line = get_line(&mut files, &msg);

//                 println!(
//                     "\n{}:{}:{}\n \x1b[33mWarning\x1b[0m: {}\n\t{}",
//                     msg.file_name,
//                     msg.line,
//                     msg.column,
//                     msg.message,
//                     line.unwrap_or_default(),
//                 )
//             }

//             MistDiagnostic::Rust(rs) => println!("{rs}"),
//         }
//     }
// }

pub fn get_line(
    files: &mut HashMap<PathBuf, Vec<String>>,
    msg: &MistDiagnosticMessage,
) -> Option<String> {
    let src_path = msg.file_path.clone();

    let lines = files.entry(src_path.clone()).or_insert_with(|| {
        fs::read_to_string(src_path)
            .expect("Unable to read mist file")
            .lines()
            .into_iter()
            .map(String::from)
            .collect()
    });

    lines.get(msg.line - 1).map(|v| v.trim().to_string())
}
