use std::{collections::HashMap, path::PathBuf, process::Stdio, sync::Arc, time::Duration};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    sync::{Mutex, oneshot},
    time::timeout,
};
use tower_lsp::lsp_types::{
    self, ClientCapabilities, InitializeParams, InitializedParams, Url, WorkspaceFolder,
    notification::{Initialized, Notification},
    request::{self, Request},
};

const MAX_CONTENT_LENGTH: usize = 50 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Deserialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub result: Option<T>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

type PendingMap = Arc<Mutex<HashMap<usize, oneshot::Sender<Result<Value, String>>>>>;

#[derive(Debug)]
pub struct RustAnalyzer {
    stdin: Arc<Mutex<tokio::process::ChildStdin>>,
    pending: PendingMap,
    id: usize,
    _child: tokio::process::Child,
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
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut line = String::new();
    let mut content_length = 0;
    let mut eof = false;

    for _ in 0..100 {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            eof = true;
            break;
        }
        if line.trim().is_empty() {
            break;
        }
        if line.to_ascii_lowercase().starts_with("content-length:") {
            if let Some(val_str) = line.split(':').nth(1) {
                let trimmed = val_str.trim();
                if !trimmed.is_empty() {
                    content_length = trimmed.parse::<usize>()?;
                }
            }
        }
    }

    if content_length == 0 {
        if eof {
            return Err("LSP stream closed (EOF)".into());
        }
        return Err("Missing, invalid, or zero Content-Length header".into());
    }
    if content_length > MAX_CONTENT_LENGTH {
        return Err(format!(
            "Content-Length {} exceeds maximum threshold",
            content_length
        )
        .into());
    }

    let mut buffer = vec![0u8; content_length];
    reader.read_exact(&mut buffer).await?;

    Ok(String::from_utf8(buffer)?)
}

impl RustAnalyzer {
    pub fn new(
        notification_tx: tokio::sync::mpsc::UnboundedSender<Value>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut child = tokio::process::Command::new("rustup")
            .args(["run", "stable", "rust-analyzer"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or("Failed to open child stdin pipe")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Failed to open child stdout pipe")?;
        let stderr = child
            .stderr
            .take()
            .ok_or("Failed to open child stderr pipe")?;

        tokio::spawn(async move {
            let mut stderr = stderr;
            let mut buf = vec![0u8; 4096];
            loop {
                match stderr.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        eprintln!(
                            "[ra-stderr] {}",
                            String::from_utf8_lossy(&buf[..n]).trim_end()
                        );
                    }
                    Err(e) => {
                        eprintln!("[ra-stderr] read error: {e}");
                        break;
                    }
                }
            }
            eprintln!("[ra-stderr] stream closed");
        });

        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let pending_clone = pending.clone();

        tokio::spawn(async move {
            let mut stdout = BufReader::new(stdout);
            eprintln!("[ra-reader] started");

            loop {
                let raw = match read_lsp_message(&mut stdout).await {
                    Ok(v) => v,
                    Err(err) => {
                        eprintln!("LSP stream error: {err}");
                        // Drain pending requests so they don't hang forever.
                        let mut lock = pending_clone.lock().await;
                        for (_, tx) in lock.drain() {
                            let _ = tx.send(Err(format!("LSP reader error: {}", err)));
                        }
                        // EOF means the child process exited — stop the reader.
                        // Other errors (malformed frames) get a brief backoff then retry.
                        if err.to_string().contains("LSP stream closed (EOF)") {
                            eprintln!("LSP reader task: EOF detected, stopping reader");
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        continue;
                    }
                };

                let value: Value = match serde_json::from_str(&raw) {
                    Ok(v) => v,
                    Err(err) => {
                        eprintln!("Corrupted JSON received: {err}");
                        continue;
                    }
                };

                let is_notification = value.get("method").is_some() && value.get("id").is_none();

                if is_notification {
                    let _ = notification_tx.send(value);
                    continue;
                }

                let id = value.get("id").and_then(|v| v.as_u64()).map(|v| v as usize);

                if let Some(id) = id {
                    let tx = pending_clone.lock().await.remove(&id);
                    if let Some(tx) = tx {
                        let _ = tx.send(Ok(value));
                    } else {
                        eprintln!("Received orphaned or delayed frame for ID: {id}");
                    }
                } else {
                    eprintln!("Received unhandled protocol notification framework: {raw}");
                }
            }
        });

        Ok(Self {
            stdin: Arc::new(Mutex::new(stdin)),
            pending,
            id: 0,
            _child: child,
        })
    }

    pub async fn request<R: Request>(
        &mut self,
        params: R::Params,
    ) -> Result<R::Result, Box<dyn std::error::Error + Send + Sync>>
    where
        R::Result: DeserializeOwned,
    {
        let id = {
            self.id += 1;
            self.id
        };

        let (tx, rx) = oneshot::channel();

        {
            self.pending.lock().await.insert(id, tx);
        }

        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": R::METHOD,
            "params": params,
        });

        let mut stdin_lock = self.stdin.lock().await;

        if let Err(err) = send_lsp_message(&mut *stdin_lock, &payload).await {
            self.pending.lock().await.remove(&id);
            return Err(Box::new(err));
        }

        drop(stdin_lock);

        let response_payload = match timeout(REQUEST_TIMEOUT, rx).await {
            Ok(Ok(Ok(value))) => value,
            Ok(Ok(Err(task_err))) => return Err(task_err.into()),
            Ok(Err(_oneshot_canceled)) => {
                return Err(
                    "Bridge connection closed down; reader channel dropped unexpectedly".into(),
                );
            }
            Err(_timeout_elapsed) => {
                self.pending.lock().await.remove(&id);
                return Err(
                    format!("Request ID {} timed out after {:?}", id, REQUEST_TIMEOUT).into(),
                );
            }
        };

        let envelope: JsonRpcResponse<R::Result> =
            serde_json::from_value(response_payload.clone())?;

        if let Some(err) = envelope.error {
            return Err(format!("LSP Engine Error ({}): {}", err.code, err.message).into());
        }

        envelope.result.ok_or_else(|| {
            format!(
                "Missing inner structural payload result: {:?}",
                response_payload
            )
            .into()
        })
    }

    pub async fn notify<T: Serialize>(
        &mut self,
        method: &str,
        req: T,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": req,
        });

        let mut stdin_lock = self.stdin.lock().await;
        send_lsp_message(&mut *stdin_lock, &payload).await?;
        Ok(())
    }
}

impl RustAnalyzer {
    pub async fn initialize(
        &mut self,
        root: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

        self.request::<request::Initialize>(init_params).await?;
        Ok(())
    }

    pub async fn initialized(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.notify(<Initialized as Notification>::METHOD, InitializedParams {})
            .await?;
        Ok(())
    }
}

impl RustAnalyzer {
    pub async fn did_open(
        &mut self,
        uri: Url,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "rust",
                    "version": 1,
                    "text": text,
                }
            }),
        )
        .await
    }

    pub async fn did_change(
        &mut self,
        uri: Url,
        text: &str,
        version: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.notify(
            "textDocument/didChange",
            json!({
                "textDocument": {
                    "uri": uri,
                    "version": version,
                },
                "contentChanges": [{
                    "text": text,
                }],
            }),
        )
        .await
    }

    pub async fn did_close(
        &mut self,
        uri: Url,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.notify(
            "textDocument/didClose",
            json!({
                "textDocument": {
                    "uri": uri,
                }
            }),
        )
        .await
    }
}
