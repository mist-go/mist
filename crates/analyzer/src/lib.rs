pub mod rust_analyzer;

use std::collections::{HashMap, HashSet};
use std::path::{Component, PathBuf};
use std::sync::Arc;

use mist_parser::rev_mapper;
use ropey::Rope;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{self, *};
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::rust_analyzer::RustAnalyzer;

#[derive(Debug)]
struct Backend {
    client: Client,
    workspace_folder: Arc<Mutex<Option<PathBuf>>>,
    previous_diagnostics: Arc<Mutex<HashMap<Url, Vec<Diagnostic>>>>,
    rust_analyzer: Arc<Mutex<RustAnalyzer>>,
    mapping: Arc<Mutex<HashMap<PathBuf, HashSet<(rev_mapper::RustMap, rev_mapper::MistMap)>>>>,
    documents: Arc<Mutex<HashMap<PathBuf, Rope>>>,
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
                rs_loc
                // FIX: Match the find_mapping operation instead of calling .expect("Failed to map")
                // match rev_mapper::find_mapping(
                //     mapping,
                //     &rev_mapper::RustMap(
                //         rs_loc.range.start.line as usize,
                //         rs_loc.range.start.character as usize,
                //     ),
                // ) {
                //     Some((_, rev_mapper::MistMap(line, character))) => Location {
                //         uri: Url::from_file_path(from_rust_to_mist(rs_path)).unwrap(),
                //         range: Range {
                //             start: Position {
                //                 line: line as u32 - 1,
                //                 character: character as u32,
                //             },
                //             end: Position {
                //                 line: line as u32 - 1,
                //                 character: character as u32 + 1,
                //             },
                //         },
                //     },
                //     None => rs_loc,
                // }
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

        let documents = self.documents.clone();

        let mapping = self.mapping.clone();

        tokio::spawn(async move {
            if let Some(root) = &*workspace_folder.lock().await {
                let src_root = root.join("src");

                analyzer
                    .lock()
                    .await
                    .initialize(root)
                    .await
                    .expect("Failed to initialize rust analyzer");

                // ---- LOAD ALL .MIST FILES ----
                let mut files = Vec::new();
                collect_mist_files(&src_root, &mut files);

                for file in files {
                    if let Ok(text) = std::fs::read_to_string(&file) {}
                }

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
        documents: Arc::new(Mutex::new(HashMap::new())),
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
        let replacement = std::path::Path::new(".mist/src");
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

    let pattern: Vec<Component> = std::path::Path::new(".mist/src").components().collect();

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

fn insert_at_position(rope: &Rope, line: usize, col: usize, insert: &str) -> Rope {
    let mut rope = rope.clone();

    let line_idx = line.saturating_sub(1);
    let col_idx = col.saturating_sub(1);

    let line_idx = line_idx.min(rope.len_lines().saturating_sub(1));

    let line_start = rope.line_to_char(line_idx);

    let line_slice = rope.line(line_idx);
    let line_len = line_slice.len_chars();

    let col_idx = col_idx.min(line_len);

    let idx = line_start + col_idx;

    rope.insert(idx, insert);

    rope
}

fn find_row_col(rope: &Rope, needle: &str) -> Option<(usize, usize)> {
    let text = rope.to_string();

    let byte_idx = text.find(needle)?;

    let char_idx = text[..byte_idx].chars().count();

    let line_idx = rope.char_to_line(char_idx);

    let line_start = rope.line_to_char(line_idx);

    let col_idx = char_idx - line_start;

    Some((line_idx, col_idx + 1))
}

fn simplify_item(mut item: CompletionItem) -> CompletionItem {
    item.text_edit = None;
    item.additional_text_edits = None;
    item.command = None;

    item
}

fn collect_mist_files(dir: &PathBuf, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            collect_mist_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("mist") {
            out.push(path);
        }
    }
}
