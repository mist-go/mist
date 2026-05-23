pub mod builder;
pub mod transpiler;

use std::collections::HashMap;
use std::fs;
use std::path::{Component, PathBuf};
use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::builder::MistDiagnostic;

#[derive(Debug)]
struct Backend {
    client: Client,
    workspace_folder: Arc<Mutex<Option<PathBuf>>>,
    previous_diagnostics: Arc<Mutex<HashMap<Url, Vec<Diagnostic>>>>,
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

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let mut res = InitializeResult::default();

        res.capabilities.text_document_sync = Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::INCREMENTAL),
                will_save: Some(false),
                will_save_wait_until: Some(false),
                save: Some(SaveOptions::default().into()),
            },
        ));

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
        tokio::spawn(async move {
            if let Some(root) = &*workspace_folder.lock().await {
                transpiler::build(root);
            }
        });

        Ok(res)
    }

    async fn initialized(&self, _: InitializedParams) {
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

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let mut source = fs::read_to_string(
            params
                .text_document_position_params
                .text_document
                .uri
                .to_file_path()
                .unwrap(),
        )
        .expect("Failed to read source");

        let inject = "__mist_23";

        source = insert_at_position(
            &source,
            params.text_document_position_params.position.line as usize,
            params.text_document_position_params.position.character as usize,
            &inject,
        );

        let output = transpiler::transpile_text(&source).expect("Failed to transpile");

        let position = output.find(inject).expect("Didn't find injection").saturating_sub(1);

        self.client
            .log_message(MessageType::INFO, format!("Found at {position}"))
            .await;

        Ok(None)
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

fn insert_at_position(s: &str, line: usize, col: usize, insert: &str) -> String {
    let mut lines: Vec<String> = s.lines().map(|l| l.to_string()).collect();

    // Convert to 0-index
    let line_idx = line.saturating_sub(1);
    let col_idx = col.saturating_sub(1);

    // Ensure line exists (optional behavior: extend with empty lines)
    if line_idx >= lines.len() {
        lines.resize(line_idx + 1, String::new());
    }

    let target = &mut lines[line_idx];

    // Clamp column to valid char boundary
    let char_idx = target
        .char_indices()
        .nth(col_idx)
        .map(|(i, _)| i)
        .unwrap_or(target.len()); // if past end, append

    target.insert_str(char_idx, insert);

    lines.join("\n")
}
