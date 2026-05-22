pub mod builder;
pub mod transpiler;

use std::path::{Component, PathBuf};
use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::builder::MistDiagnostic;
use crate::transpiler::transpile_file;

#[derive(Debug)]
struct Backend {
    client: Client,
    workspace_folder: Arc<Mutex<Option<PathBuf>>>,
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

        // Safely extract workspace root without deep nested matching panics
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
                // Consideration: Ensure transpiler::build is panic-safe internally
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

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "Processing did_save event")
            .await;

        // 1. Safe URI parsing fallback
        let input_path = match params.text_document.uri.to_file_path() {
            Ok(path) => path,
            Err(_) => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        "Skipping: Document URI is not a valid local file path",
                    )
                    .await;
                return;
            }
        };

        let output_path = from_mist_to_rust(input_path.clone());

        self.client
            .log_message(
                MessageType::INFO,
                format!("Transpiling: {}", input_path.display()),
            )
            .await;

        // 2. Safe call to the updated transpile_file function
        if let Err(err_msg) = transpile_file(&input_path, &output_path) {
            self.client
                .log_message(
                    MessageType::ERROR,
                    format!("Transpilation error: {err_msg}"),
                )
                .await;
            // Optimization choice: You can return early here, or keep moving forward
            // to collect compiler diagnostics anyway.
        }

        // 3. Safe workspace root retrieval fallback
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

        // 4. Run the project builder stage
        let diagnostics_raw = builder::build(
            vec![
                "check".to_string(),
                "--bin".to_string(),
                "mist-lsp".to_string(),
            ],
            workspace_root,
        );

        let mut diagnostics = Vec::new();

        for diag in diagnostics_raw.iter() {
            let (msg, severity) = match diag {
                MistDiagnostic::Error(msg) => (msg, DiagnosticSeverity::ERROR),
                MistDiagnostic::Warning(msg) => (msg, DiagnosticSeverity::WARNING),
                MistDiagnostic::Rust(d) => {
                    self.client
                        .log_message(
                            MessageType::LOG,
                            format!("Unhandled underlying Rust diagnostic: {d:?}"),
                        )
                        .await;
                    continue;
                }
            };

            // 5. Safe indexing checks via saturating_sub
            let line = (msg.line as u32).saturating_sub(1);
            let column = (msg.column as u32).saturating_sub(1);

            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line,
                        character: column,
                    },
                    end: Position {
                        line,
                        character: u32::MAX, // Highlights to the end of the line safely
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
            });
        }

        // 6. Send the generated diagnostics back to the IDE client interface
        self.client
            .publish_diagnostics(params.text_document.uri, diagnostics, None)
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
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

fn from_mist_to_rust(mut path: PathBuf) -> PathBuf {
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
