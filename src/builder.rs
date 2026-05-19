use std::{
    collections::{HashMap, HashSet},
    fs,
    process::{Command, Stdio},
};

use cargo_metadata::Message;
use mist_parser::rev_mapper::{MistMap, RustMap, find_mapping, get_mapping};

#[derive(Debug, Clone)]
pub struct MistDiagnosticMessage {
    pub message: String,
    pub src_path: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub enum MistDiagnostic {
    Error(MistDiagnosticMessage),
}

pub fn build(mut args: Vec<String>) -> Result<Vec<MistDiagnostic>, Vec<MistDiagnostic>> {
    args.remove(0);
    args.insert(1, "--message-format=json".to_string());

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
                let file_name = msg.target.src_path.as_str().to_string().clone();

                let map = mapping
                    .entry(file_name.clone())
                    .or_insert_with(|| get_mapping(&fs::read_to_string(file_name).unwrap()));

                for span in msg.message.spans {
                    let mist_span =
                        find_mapping(&map, &RustMap(span.line_end, span.column_start)).unwrap();

                    diagnostics.push(MistDiagnostic::Error(MistDiagnosticMessage {
                        message: span.label.unwrap_or(msg.message.message.clone()),
                        src_path: span.file_name,
                        line: mist_span.1.0,
                        column: mist_span.1.1,
                    }));
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
