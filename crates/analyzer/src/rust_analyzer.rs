use std::{path::PathBuf, process::Stdio};

use lsp_types::{ClientCapabilities, InitializeParams, InitializeResult, Url, WorkspaceFolder};
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

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

pub async fn initialize(root: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Spawn rust-analyzer
    let mut child = tokio::process::Command::new("rust-analyzer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null()) // Ignore logs for simplicity
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    let project_uri = Url::from_directory_path(root)
        .map_err(|_| "Failed to convert path to valid file:// URL")?;

    #[allow(deprecated)]
    let init_params = InitializeParams {
        process_id: Some(std::process::id()), // Highly recommended so RA knows if your client dies
        root_uri: Some(project_uri.clone()),  // Provide root_uri as a fallback alongside folders
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

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": init_params
    });

    send_lsp_message(&mut stdin, &init_request).await?;
    eprintln!("-> Sent: initialize");

    // Step 3: Read 'initialize' response
    let response_str = read_lsp_message(&mut stdout).await?;

    // DEBUG: If it fails again, this print will show you exactly what rust-analyzer sent back!
    let json_parsed: serde_json::Value = serde_json::from_str(&response_str)?;
    if let Some(error) = json_parsed.get("error") {
        panic!(
            "Rust-analyzer explicitly rejected our request! Error: {:#?}",
            error
        );
    }

    // If no explicit error, try parsing into the strong type
    let _response: InitializeResult =
        serde_json::from_value(json_parsed.get("result").cloned().unwrap_or(json_parsed))?;
    eprintln!("<- Received: initialize response");

    Ok(())
}
