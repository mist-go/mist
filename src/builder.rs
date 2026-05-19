use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::PathBuf,
    process::{Command, Stdio},
};

use cargo_metadata::{CompilerMessage, Message};
use mist_parser::rev_mapper::{MistMap, RustMap, find_mapping, get_mapping};

use crate::transpiler::Config;

#[derive(Debug, Clone)]
pub struct MistDiagnosticMessage {
    pub message: String,
    pub file_path: String,
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

pub fn build(
    mut args: Vec<String>,
    config: Config,
    root: PathBuf,
) -> Result<Vec<MistDiagnostic>, Vec<MistDiagnostic>> {
    args.remove(0);
    args.insert(1, "--message-format=json".to_string());

    let is_root = root == env::current_dir().unwrap();

    let mut command = Command::new("cargo")
        .args(args)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let reader = std::io::BufReader::new(command.stdout.take().unwrap());

    let mut diagnostics = Vec::new();

    let mut mapping: HashMap<String, HashSet<(RustMap, MistMap)>> = HashMap::new();

    for message in cargo_metadata::Message::parse_stream(reader) {
        match message {
            Ok(Message::CompilerMessage(msg)) => {
                for span in &msg.message.spans {
                    if span.is_primary {
                        let file_name = span.file_name.clone();

                        let mist_file = span
                            .file_name
                            .replacen(&config.output, &config.src, 1)
                            .replace(".rs", ".mist");

                        let mist_path = root.join(&mist_file);

                        if !fs::exists(&mist_path).unwrap() {
                            diagnostics.push(MistDiagnostic::Rust(msg));
                            break;
                        }

                        let map = mapping.entry(file_name.clone()).or_insert_with(|| {
                            get_mapping(&fs::read_to_string(file_name).unwrap())
                        });

                        let mist_span =
                            find_mapping(&map, &RustMap(span.line_end, span.column_start)).unwrap();

                        let mist_msg = MistDiagnosticMessage {
                            message: span.label.clone().unwrap_or(msg.message.message.clone()),
                            file_path: mist_path.to_string_lossy().to_string(),
                            file_name: if is_root {
                                mist_file
                            } else {
                                mist_path.to_string_lossy().to_string()
                            },
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

            Ok(Message::BuildFinished(finish)) => {
                if finish.success {
                    return Ok(diagnostics);
                } else {
                    return Err(diagnostics);
                }
            }
            _ => {}
        }
    }

    Ok(diagnostics)
}

pub fn print_diagnostics(diagnostics: Vec<MistDiagnostic>) {
    let mut files = HashMap::new();

    for diag in diagnostics {
        match diag {
            MistDiagnostic::Error(msg) => {
                let line = get_line(&mut files, &msg);

                println!(
                    "\n{}:{}:{}\n \x1b[31mError\x1b[0m: {}\n\t{}",
                    msg.file_name,
                    msg.line + 1,
                    msg.column,
                    msg.message,
                    line.unwrap_or_default(),
                )
            }

            MistDiagnostic::Warning(msg) => {
                let line = get_line(&mut files, &msg);

                println!(
                    "\n{}:{}:{}\n \x1b[33mWarning\x1b[0m: {}\n\t{}",
                    msg.file_name,
                    msg.line + 1,
                    msg.column,
                    msg.message,
                    line.unwrap_or_default(),
                )
            }
            MistDiagnostic::Rust(rs) => println!("{rs}"),
        }
    }
}

pub fn get_line(
    files: &mut HashMap<String, Vec<String>>,
    msg: &MistDiagnosticMessage,
) -> Option<String> {
    let src_path = msg.file_path.clone();

    let lines = files.entry(src_path.clone()).or_insert_with(|| {
        fs::read_to_string(src_path)
            .unwrap()
            .lines()
            .into_iter()
            .map(String::from)
            .collect()
    });

    lines.get(msg.line).map(|v| v.trim().to_string())
}
