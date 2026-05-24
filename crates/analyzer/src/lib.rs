pub mod builder;
pub mod rust_analyzer;
pub mod transpiler;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, PathBuf};
use std::sync::Arc;

use mist_parser::rev_mapper;
use ropey::Rope;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::notification::Notification;
use tower_lsp::lsp_types::{self, *};
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::builder::MistDiagnostic;
use crate::rust_analyzer::RustAnalyzer;

#[derive(Debug)]
struct Backend {
    client: Client,
    workspace_folder: Arc<Mutex<Option<PathBuf>>>,
    previous_diagnostics: Arc<Mutex<HashMap<Url, Vec<Diagnostic>>>>,
    rust_analyzer: Arc<Mutex<RustAnalyzer>>,
    mapping: Arc<Mutex<HashMap<PathBuf, HashSet<(rev_mapper::RustMap, rev_mapper::MistMap)>>>>,
}

/// Helper function to force percent-encoding on Windows drive colons
/// so that the URLs exactly match what VS Code/LSP clients send.
fn clean_lsp_url(path: &std::path::Path) -> Option<Url> {
    let mut url_str = Url::from_file_path(path).ok()?.to_string();

    // Look for Windows patterns like file:///D: or file:///d: and switch to %3A
    if url_str.starts_with("file:///I:") || url_str.starts_with("file:///i:") || // Catch-all or explicit check:
       (url_str.len() > 10 && url_str.as_bytes()[11] == b':')
    {
        // Safely replace the first colon occurring after "file:///"
        if let Some(pos) = url_str.find(':') {
            if pos == 11 {
                // Double check it's the drive letter colon
                url_str.replace_range(pos..=pos, "%3A");
            }
        }
    }

    Url::parse(&url_str).ok()
}

impl Backend {
    async fn get_mist_location(&self, rs_loc: Location) -> Location {
        let rs_path = match rs_loc.uri.to_file_path() {
            Ok(path) => path,
            Err(_) => return rs_loc,
        };

        match self.mapping.lock().await.get(&rs_path) {
            Some(mapping) => {
                // FIX: Match the find_mapping operation instead of calling .expect("Failed to map")
                match rev_mapper::find_mapping(
                    mapping,
                    &rev_mapper::RustMap(
                        rs_loc.range.start.line as usize,
                        rs_loc.range.start.character as usize,
                    ),
                ) {
                    Some((_, rev_mapper::MistMap(line, character))) => Location {
                        uri: Url::from_file_path(from_rust_to_mist(rs_path)).unwrap(),
                        range: Range {
                            start: Position {
                                line: line as u32 - 1,
                                character: character as u32,
                            },
                            end: Position {
                                line: line as u32 - 1,
                                character: character as u32 + 1,
                            },
                        },
                    },
                    None => rs_loc,
                }
            }
            None => rs_loc,
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let mut res = InitializeResult::default();

        res.capabilities.text_document_sync = Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                will_save: Some(false),
                will_save_wait_until: Some(false),
                save: Some(SaveOptions::default().into()),
            },
        ));

        res.capabilities.completion_provider = Some(CompletionOptions {
            resolve_provider: Some(true),
            trigger_characters: Some(vec![
                ":".to_owned(),
                ".".to_owned(),
                "'".to_owned(),
                "(".to_owned(),
            ]),
            all_commit_characters: None,
            completion_item: Some(CompletionOptionsCompletionItem {
                label_details_support: None,
            }),
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
        });

        res.capabilities.definition_provider = Some(OneOf::Left(true));

        let folder_path = params
            .workspace_folders
            .as_ref()
            .and_then(|folders| folders.first())
            .and_then(|folder| folder.uri.to_file_path().ok());

        if let Some(ref path) = folder_path {
            *self.workspace_folder.lock().await = Some(path.clone());
        }

        let workspace_folder = self.workspace_folder.clone();

        let analyzer = self.rust_analyzer.clone();

        tokio::spawn(async move {
            if let Some(root) = &*workspace_folder.lock().await {
                transpiler::build(root);

                analyzer
                    .lock()
                    .await
                    .initialize(root)
                    .await
                    .expect("Failed to initialize rust analyzer");

                eprintln!("Ready to use");
            }
        });

        Ok(res)
    }

    async fn initialized(&self, _: InitializedParams) {
        self.rust_analyzer
            .lock()
            .await
            .initialized()
            .await
            .expect("Failed to initialize rust analyzer");

        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "Processing did_save event")
            .await;

        if let Some(root) = &*self.workspace_folder.lock().await {
            transpiler::build(root);
        }

        let workspace_root = match self.workspace_folder.lock().await.clone() {
            Some(root) => root,
            None => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        "Skipping diagnostics: No active workspace folder found",
                    )
                    .await;
                return;
            }
        };

        let diagnostics_raw = builder::build(
            vec![
                "check".to_string(),
                "--workspace".to_string(),
                "--all-targets".to_string(),
            ],
            workspace_root,
        );

        let mut diagnostics = HashMap::new();

        for diag in diagnostics_raw.iter() {
            let (msg, severity) = match diag {
                MistDiagnostic::Error(msg) => (msg, DiagnosticSeverity::ERROR),
                MistDiagnostic::Warning(msg) => (msg, DiagnosticSeverity::WARNING),
                MistDiagnostic::Rust(_) => {
                    continue;
                }
            };

            let line = (msg.line as u32).saturating_sub(1);
            let column = (msg.column as u32).saturating_sub(1);

            let url = match clean_lsp_url(&msg.file_path) {
                Some(u) => u,
                None => continue,
            };

            let diagnostic_item = Diagnostic {
                range: Range {
                    start: Position {
                        line,
                        character: column,
                    },
                    end: Position {
                        line,
                        character: column + 1,
                    },
                },
                severity: Some(severity),
                code: None,
                source: Some("mist-analyzer".to_string()),
                message: msg.message.clone(),
                related_information: None,
                tags: None,
                data: None,
                code_description: None,
            };

            diagnostics
                .entry(url)
                .or_insert_with(Vec::new)
                .push(diagnostic_item);
        }

        for (file, _) in self.previous_diagnostics.lock().await.iter() {
            self.client
                .publish_diagnostics(file.clone(), Vec::new(), None)
                .await;
        }

        for (file, diag) in &diagnostics {
            self.client
                .publish_diagnostics(file.clone(), diag.clone(), None)
                .await;
        }

        *self.previous_diagnostics.lock().await = diagnostics;
    }

    async fn did_open(&self, mut params: DidOpenTextDocumentParams) {
        if params.text_document.language_id == "mist" {
            params.text_document.language_id = "rust".to_string();

            match transpiler::transpile_text(&params.text_document.text) {
                Ok(transpiled_text) => {
                    params.text_document.text = transpiled_text;
                    let rust_path =
                        from_mist_to_rust(params.text_document.uri.to_file_path().unwrap());
                    params.text_document.uri = Url::from_file_path(&rust_path).unwrap();

                    self.mapping.lock().await.insert(
                        rust_path,
                        rev_mapper::get_mapping(&params.text_document.text),
                    );
                }
                Err(e) => {
                    self.client
                        .log_message(MessageType::WARNING, format!("MIST-LSP: Syntax invalid during open/change. Parsing stopped: {:?}", e))
                        .await;
                    return;
                }
            }
        }

        if let Ok(mut ra) = self.rust_analyzer.try_lock() {
            let _ = ra
                .notify(notification::DidOpenTextDocument::METHOD, params)
                .await;
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let file_path = params
            .text_document_position_params
            .text_document
            .uri
            .to_file_path()
            .unwrap();

        let source = match fs::read_to_string(&file_path) {
            Ok(src) => src,
            Err(_) => return Ok(None),
        };

        let inject = "__mist_23";
        let injected_source = insert_at_position(
            &source,
            params.text_document_position_params.position.line as usize + 1,
            params.text_document_position_params.position.character as usize,
            &inject,
        );

        let output = match transpiler::transpile_text(&injected_source) {
            Ok(out) => out,
            Err(_) => return Ok(None),
        };

        let (line, character) = match find_row_col(&output, inject) {
            Some(coords) => coords,
            None => return Ok(None),
        };

        let uri =
            Url::from_file_path(from_mist_to_rust(file_path)).expect("failed to generate rs url");

        let rs_res = self
            .rust_analyzer
            .lock()
            .await
            .request::<request::GotoDefinition>(lsp_types::GotoDefinitionParams {
                text_document_position_params: lsp_types::TextDocumentPositionParams {
                    position: lsp_types::Position {
                        line: line as u32 - 1,
                        character: character as u32,
                    },
                    text_document: lsp_types::TextDocumentIdentifier { uri },
                },
                partial_result_params: lsp_types::PartialResultParams::default(),
                work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
            })
            .await
            .expect("Failed to send to rust");

        Ok(match rs_res {
            Some(GotoDefinitionResponse::Array(arr)) => {
                let mut mapped_arr = Vec::new();
                for rs_loc in arr {
                    let mapped_loc = self.get_mist_location(rs_loc).await;
                    mapped_arr.push(mapped_loc);
                }
                Some(GotoDefinitionResponse::Array(mapped_arr))
            }
            _ => rs_res,
        })
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "MIST-LSP: Processing did_change event")
            .await;

        let rust_path = from_mist_to_rust(params.text_document.uri.to_file_path().unwrap());
        let rust_uri = Url::from_file_path(&rust_path).unwrap();

        if let Some(change) = params.content_changes.first_mut() {
            match transpiler::transpile_text(&change.text) {
                Ok(transpiled_text) => {
                    change.text = transpiled_text;

                    self.mapping
                        .lock()
                        .await
                        .insert(rust_path, rev_mapper::get_mapping(&change.text));
                }
                Err(e) => {
                    self.client
                        .log_message(
                            MessageType::WARNING,
                            format!(
                                "MIST-LSP: Syntax invalid during change. Sync stopped: {:?}",
                                e
                            ),
                        )
                        .await;
                    return;
                }
            }
        }

        // 3. Re-route URI path to mirror directory
        params.text_document.uri = rust_uri;

        // 4. Notify downstream rust-analyzer
        if let Ok(mut ra) = self.rust_analyzer.try_lock() {
            let _ = ra
                .notify(notification::DidChangeTextDocument::METHOD, params)
                .await;
        }
    }

    async fn did_close(&self, mut params: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "MIST-LSP: Processing did_close event")
            .await;

        let rust_path = from_mist_to_rust(params.text_document.uri.to_file_path().unwrap());

        // 1. Evict the mapping from memory to prevent leaks
        self.mapping.lock().await.remove(&rust_path);

        // 2. Re-route URI path to mirror directory
        params.text_document.uri = Url::from_file_path(&rust_path).unwrap();

        // 3. Notify downstream rust-analyzer
        if let Ok(mut ra) = self.rust_analyzer.try_lock() {
            let _ = ra
                .notify(notification::DidCloseTextDocument::METHOD, params)
                .await;
        }
    }

    async fn completion(&self, mut params: CompletionParams) -> Result<Option<CompletionResponse>> {
        self.client
            .log_message(MessageType::INFO, "COMPLETEING!")
            .await;

        let file_path = params
            .text_document_position
            .text_document
            .uri
            .to_file_path()
            .unwrap();

        let source = match fs::read_to_string(&file_path) {
            Ok(src) => src,
            Err(_) => return Ok(None),
        };

        let inject = "__mist_23";
        let injected_source = insert_at_position(
            &source,
            params.text_document_position.position.line as usize + 1,
            params.text_document_position.position.character as usize,
            &inject,
        );

        let output = match transpiler::transpile_text(&injected_source) {
            Ok(out) => out,
            Err(_) => return Ok(None),
        };

        let (line, character) = match find_row_col(&output, inject) {
            Some(coords) => coords,
            None => return Ok(None),
        };

        let uri =
            Url::from_file_path(from_mist_to_rust(file_path)).expect("failed to generate rs url");

        params.text_document_position.text_document.uri = uri;
        params.text_document_position.position.line = line as u32;
        params.text_document_position.position.character = character as u32;

        let rs_res = self
            .rust_analyzer
            .lock()
            .await
            .request::<request::Completion>(params)
            .await
            .expect("Failed to send to rust");

        Ok(rs_res)
    }
}

#[tokio::main]
pub async fn start() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        workspace_folder: Arc::new(Mutex::new(None)),
        previous_diagnostics: Arc::new(Mutex::new(HashMap::new())),
        mapping: Arc::new(Mutex::new(HashMap::new())),
        rust_analyzer: Arc::new(Mutex::new(
            RustAnalyzer::new().expect("Failed to create rust analyzer"),
        )),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

pub fn from_mist_to_rust(mut path: PathBuf) -> PathBuf {
    path.set_extension("rs");

    let mut comps: Vec<Component> = path.components().collect();

    if let Some(pos) = comps.iter().rposition(|c| c.as_os_str() == "src") {
        let replacement = std::path::Path::new(".mist/lsp");
        comps.splice(pos..=pos, replacement.components());
        comps.iter().collect()
    } else {
        path
    }
}

pub fn from_rust_to_mist(mut path: PathBuf) -> PathBuf {
    // reverse extension
    path.set_extension("mist");

    let comps: Vec<Component> = path.components().collect();

    // pattern we originally inserted: ".mist/lsp"
    let pattern: Vec<Component> = std::path::Path::new(".mist/lsp").components().collect();

    // find the last occurrence of the pattern
    if let Some(pos) = comps
        .windows(pattern.len())
        .rposition(|window| window == pattern.as_slice())
    {
        let mut new_comps = comps.clone();

        // replace the matched range with "src"
        new_comps.splice(
            pos..pos + pattern.len(),
            std::iter::once(Component::Normal(std::ffi::OsStr::new("src"))),
        );

        new_comps.iter().collect()
    } else {
        path
    }
}

fn insert_at_position(s: &str, line: usize, col: usize, insert: &str) -> String {
    let mut rope = Rope::from_str(s);

    // Convert to 0-indexed
    let line_idx = line.saturating_sub(1);
    let col_idx = col.saturating_sub(1);

    // Clamp line
    let line_idx = line_idx.min(rope.len_lines().saturating_sub(1));

    // Get start char index of the line
    let line_start = rope.line_to_char(line_idx);

    // Get line length in chars
    let line = rope.line(line_idx);
    let line_len = line.len_chars();

    // Clamp column
    let col_idx = col_idx.min(line_len);

    // Final insertion char index
    let insert_idx = line_start + col_idx;

    rope.insert(insert_idx, insert);

    rope.to_string()
}

fn find_row_col(output: &str, inject: &str) -> Option<(usize, usize)> {
    let rope = Rope::from_str(output);

    let byte_idx = output.find(inject)?;

    // Convert byte index -> char index
    let char_idx = output[..byte_idx].chars().count();

    // Ropey line lookup
    let line_idx = rope.char_to_line(char_idx);

    // Line start char index
    let line_start = rope.line_to_char(line_idx);

    // Column within line
    let col_idx = char_idx - line_start;

    // Return 1-indexed
    Some((line_idx + 1, col_idx + 1))
}
