pub mod transpiler;

use std::path::{Component, PathBuf};
use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

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

        // res.capabilities.completion_provider = Some(CompletionOptions {
        //     resolve_provider: None,
        //     trigger_characters: None,
        //     all_commit_characters: None,
        //     work_done_progress_options: WorkDoneProgressOptions {
        //         work_done_progress: None,
        //     },
        //     completion_item: None,
        // });

        res.capabilities.text_document_sync = Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::INCREMENTAL),
                will_save: Some(false),
                will_save_wait_until: Some(false),
                save: Some(SaveOptions::default().into()),
            },
        ));

        *self.workspace_folder.lock().await = params
            .workspace_folders
            .iter()
            .next()
            .map(|folders| {
                folders
                    .iter()
                    .next()
                    .map(|v| v.uri.to_file_path().ok())
                    .flatten()
            })
            .flatten();

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

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "getting completion")
            .await;

        let input_path = params
            .text_document
            .uri
            .to_file_path()
            .expect("Invalid document path");

        let output_path = from_mist_to_rust(input_path.clone());

        self.client
            .log_message(MessageType::INFO, output_path.display())
            .await;

        transpile_file(&input_path, &output_path);
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

    // 1. Convert path to a vector of components
    let mut comps: Vec<Component> = path.components().collect();

    // 2. Find the index of the *last* component matching "src"
    if let Some(pos) = comps.iter().rposition(|c| c.as_os_str() == "src") {
        // Define your replacement path
        let replacement = std::path::Path::new(".mist/lsp");

        // 3. Splice the replacement components into the original vector,
        // replacing the single "src" component at `pos`
        comps.splice(pos..=pos, replacement.components());

        // 4. Rebuild the PathBuf from the modified components
        comps.iter().collect()
    } else {
        // Return path with just the extension changed if "src" wasn't found
        path
    }
}
