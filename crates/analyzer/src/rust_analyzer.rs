use std::{collections::HashMap, path::PathBuf, process::Stdio, sync::Arc};

use lsp_types::{
    ClientCapabilities, InitializeParams, InitializedParams, Url, WorkspaceFolder,
    notification::{Initialized, Notification},
    request::{self, Request},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    sync::{Mutex, mpsc::Sender},
};

#[derive(Deserialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: Option<usize>,
    pub result: Option<T>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct RustAnalyzer {
    stdin: tokio::process::ChildStdin,
    stdout: Arc<Mutex<BufReader<tokio::process::ChildStdout>>>,
    pending: Arc<Mutex<HashMap<usize, Sender<String>>>>,
    id: usize,
}

async fn send_lsp_message<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    value: &serde_json::Value,
) -> std::io::Result<()> {
    let payload = serde_json::to_string(value)?;
    let frame = format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload);
    writer.write_all(frame.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

async fn read_lsp_message<R: AsyncBufReadExt + Unpin>(
    reader: &mut R,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut line = String::new();
    let mut content_length = 0;

    // Read headers until we hit the empty separator line (\r\n)
    loop {
        line.clear();
        reader.read_line(&mut line).await?;
        if line == "\r\n" || line.is_empty() {
            break;
        }
        if line.to_lowercase().starts_with("content-length:") {
            content_length = line["content-length:".len()..].trim().parse::<usize>()?;
        }
    }

    if content_length == 0 {
        return Err("Missing or invalid Content-Length header".into());
    }

    // Read the exact byte buffer payload
    let mut buffer = vec![0u8; content_length];
    reader.read_exact(&mut buffer).await?;

    Ok(String::from_utf8(buffer)?)
}

impl RustAnalyzer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut child = tokio::process::Command::new("rust-analyzer")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()) // Ignore logs for simplicity
            .spawn()?;

        let stdin = child.stdin.take().unwrap();
        let stdout = Arc::new(Mutex::new(BufReader::new(child.stdout.take().unwrap())));

        Ok(Self {
            stdin,
            stdout,
            pending: Arc::new(Mutex::new(HashMap::new())),
            id: 0,
        })
    }

    pub async fn request<R: Request>(
        &mut self,
        params: R::Params,
    ) -> Result<R::Result, Box<dyn std::error::Error>> {
        let id = self.send(R::METHOD, params).await?;
        self.read(id).await
    }

    pub async fn send<T: Serialize>(&mut self, method: &str, req: T) -> std::io::Result<usize> {
        let id = {
            self.id += 1;
            self.id
        };

        send_lsp_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": req,
            }),
        )
        .await?;

        Ok(id)
    }

    pub async fn read<T>(&mut self, id: usize) -> Result<T, Box<dyn std::error::Error>>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        self.pending.lock().await.insert(id, tx);

        if self.pending.lock().await.len() == 1 {
            let stdout = self.stdout.clone();
            let pending = self.pending.clone();

            tokio::spawn(async move {
                let raw_string = read_lsp_message(&mut *stdout.lock().await).await.unwrap();

                let envelope: JsonRpcResponse<Value> = serde_json::from_str(&raw_string).unwrap();

                pending
                    .lock()
                    .await
                    .get(&envelope.id.unwrap())
                    .unwrap()
                    .send(raw_string)
                    .await
                    .unwrap();
            });
        }

        let raw_string = rx.recv().await.expect("Failed to receive");

        let envelope: JsonRpcResponse<T> = serde_json::from_str(&raw_string)?;

        if let Some(err) = envelope.error {
            return Err(format!("LSP Error ({}): {}", err.code, err.message).into());
        }

        envelope.result.ok_or_else(|| {
            format!(
                "LSP response missing both result and error fields. Raw: {}",
                raw_string
            )
            .into()
        })
    }

    pub async fn notify<T: Serialize>(&mut self, method: &str, req: T) -> std::io::Result<()> {
        send_lsp_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": req,
            }),
        )
        .await
    }
}

impl RustAnalyzer {
    pub async fn initialize(&mut self, root: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let project_uri = Url::from_directory_path(root)
            .map_err(|_| "Failed to convert path to valid file:// URL")?;

        #[allow(deprecated)]
        let init_params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(project_uri.clone()),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: project_uri,
                name: "workspace".to_string(),
            }]),
            capabilities: ClientCapabilities {
                workspace: Some(lsp_types::WorkspaceClientCapabilities {
                    workspace_folders: Some(true),
                    ..Default::default()
                }),
                text_document: Some(lsp_types::TextDocumentClientCapabilities {
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let response = self.request::<request::Initialize>(init_params).await;

        eprintln!("<- Received: initialize response {response:?}");

        Ok(())
    }

    pub async fn initialized(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.notify(Initialized::METHOD, InitializedParams {})
            .await?;

        Ok(())
    }
}
