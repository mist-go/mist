pub mod transpiler;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    workspace_folder: Arc<Mutex<Option<PathBuf>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let mut res = InitializeResult::default();

        res.capabilities.completion_provider = Some(CompletionOptions {
            resolve_provider: None,
            trigger_characters: None,
            all_commit_characters: None,
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
            completion_item: None,
        });

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

    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        self.client
            .log_message(MessageType::INFO, "getting completion")
            .await;

        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("new".to_string(), "The new keyword".to_string()),
        ])))
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
