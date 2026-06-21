pub mod rust_analyzer;
pub mod transpiler;

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use mist_parser::rev_mapper::{Mapping, MistMap, RustMap};
use ropey::Rope;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::Mutex;
use tower_lsp::lsp_types::{self, *};
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::rust_analyzer::RustAnalyzer;
use crate::transpiler::transpile_mist;

static MARKER_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct Backend {
    client: Client,
    workspace_folder: Arc<Mutex<Option<PathBuf>>>,
    previous_diagnostics: Arc<Mutex<HashMap<Url, Vec<Diagnostic>>>>,
    rust_analyzer: Arc<Mutex<RustAnalyzer>>,
    mapping: Arc<Mutex<HashMap<PathBuf, Mapping>>>,
    documents: Arc<Mutex<HashMap<PathBuf, Rope>>>,
    doc_versions: Arc<Mutex<HashMap<PathBuf, i32>>>,
    notification_rx: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<Value>>>>,
}

fn clean_lsp_url(path: &std::path::Path) -> Option<Url> {
    let mut url_str = Url::from_file_path(path).ok()?.to_string();

    if url_str.starts_with("file:///I:")
        || url_str.starts_with("file:///i:")
        || (url_str.len() > 10 && url_str.as_bytes()[11] == b':')
    {
        if let Some(pos) = url_str.find(':') {
            if pos == 11 {
                url_str.replace_range(pos..=pos, "%3A");
            }
        }
    }

    Url::parse(&url_str).ok()
}

fn lsp_pos_to_rust_map(pos: &Position) -> RustMap {
    RustMap(pos.line as usize + 1, pos.character as usize)
}

fn mist_map_to_lsp_pos(map: &MistMap) -> Position {
    Position {
        line: map.0.saturating_sub(1) as u32,
        character: map.1 as u32,
    }
}

fn byte_offset_from_lsp(source: &str, line: u32, character: u32) -> Option<usize> {
    let mut cur_line = 0u32;
    let mut cur_col = 0u32;
    for (i, ch) in source.char_indices() {
        if cur_line == line && cur_col == character {
            return Some(i);
        }
        if ch == '\n' {
            cur_line += 1;
            cur_col = 0;
        } else if cur_line <= line {
            cur_col += 1;
        }
    }
    if cur_line == line && cur_col == character {
        Some(source.len())
    } else {
        None
    }
}

fn find_marker_position(content: &str, marker: &str) -> Option<Position> {
    let idx = content.find(marker)?;
    let before = &content[..idx];
    let line = before.matches('\n').count() as u32;
    let last_nl = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let character = before[last_nl..].chars().count() as u32;
    Some(Position { line, character })
}

fn mist_to_rust_path(mist_path: &Path) -> PathBuf {
    let mut path = mist_path.to_path_buf();
    path.set_extension("rs");

    let comps: Vec<Component> = path.components().collect();
    if let Some(pos) = comps.iter().rposition(|c| c.as_os_str() == "src") {
        let replacement = Path::new(".mist/src");
        let mut new_comps: Vec<Component> = comps[..pos].to_vec();
        new_comps.extend(replacement.components());
        new_comps.extend(&comps[pos + 1..]);
        new_comps.iter().collect()
    } else {
        path
    }
}

fn rust_to_mist_path(rust_path: &Path) -> PathBuf {
    let mut path = rust_path.to_path_buf();
    path.set_extension("mist");

    let comps: Vec<Component> = path.components().collect();

    let pattern: Vec<Component> = Path::new(".mist/src").components().collect();

    if let Some(pos) = comps
        .windows(pattern.len())
        .rposition(|window| window == pattern.as_slice())
    {
        let mut new_comps = comps.clone();
        new_comps.splice(
            pos..pos + pattern.len(),
            std::iter::once(Component::Normal(std::ffi::OsStr::new("src"))),
        );
        new_comps.iter().collect()
    } else {
        path
    }
}

fn rust_uri_to_mist_uri(rust_uri: &Url) -> Option<Url> {
    let rust_path = rust_uri.to_file_path().ok()?;
    let mist_path = rust_to_mist_path(&rust_path);
    clean_lsp_url(&mist_path)
}

impl Backend {
    /// Resolve a Mist cursor position to a Rust position by injecting a unique marker
    /// into the source at the cursor, transpiling, and finding the marker in the output.
    async fn resolve_mist_via_marker(
        &self,
        mist_path: &Path,
        source: &str,
        line: u32,
        character: u32,
        extra_mod_decl: &str,
    ) -> Option<(Url, Position)> {
        let id = MARKER_COUNTER.fetch_add(1, Ordering::Relaxed);
        let marker = format!("__mist_mk{id:x}__");

        let offset = byte_offset_from_lsp(source, line, character)?;

        let mut modified = String::with_capacity(source.len() + marker.len() + 4);
        modified.push_str(&source[..offset]);
        modified.push(' ');
        modified.push_str(&marker);
        modified.push(' ');
        modified.push_str(&source[offset..]);

        let transpiled = transpile_mist(mist_path, &modified, extra_mod_decl).ok()?;
        let rust_pos = find_marker_position(&transpiled.rust_content, &marker)?;
        let rust_uri = clean_lsp_url(&transpiled.rust_path)?;

        Some((rust_uri, rust_pos))
    }

    async fn compute_mod_decl_for_file(&self, mist_path: &Path) -> String {
        let ws = match &*self.workspace_folder.lock().await {
            Some(p) => p.clone(),
            None => return String::new(),
        };
        let src_root = ws.join("src");
        let package_mist = read_mist_package(&ws);
        let docs = self.documents.lock().await;
        let mod_decls = compute_mod_decls(&docs, &src_root, &package_mist);
        mod_decls.get(mist_path).cloned().unwrap_or_default()
    }

    async fn map_rust_to_mist_pos(
        &self,
        rust_uri: &Url,
        rust_pos: &Position,
    ) -> Option<(Url, Position)> {
        let mist_uri = rust_uri_to_mist_uri(rust_uri)?;
        let rust_path = rust_uri.to_file_path().ok()?;

        let rust_target = lsp_pos_to_rust_map(rust_pos);
        let mapping = self.mapping.lock().await.get(&rust_path)?.clone();
        let (_, mist_map) = mapping.find(&rust_target)?;

        let pos = mist_map_to_lsp_pos(&mist_map);
        Some((mist_uri, pos))
    }

    async fn handle_transpile_and_notify(
        &self,
        mist_path: &Path,
        source: &str,
        extra_mod_decl: &str,
    ) {
        let transpiled = match transpile_mist(mist_path, source, extra_mod_decl) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("transpile error for {:?}: {e}", mist_path);
                let diag = Diagnostic {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 1,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("mist".to_string()),
                    message: format!("Transpile error: {e}"),
                    ..Default::default()
                };
                if let Some(uri) = clean_lsp_url(mist_path) {
                    self.publish_diagnostics(uri, vec![diag]).await;
                }
                return;
            }
        };

        if let Some(rust_uri) = clean_lsp_url(&transpiled.rust_path) {
            let mut ra = self.rust_analyzer.lock().await;
            let mut map = self.mapping.lock().await;
            let mut versions = self.doc_versions.lock().await;

            let version = versions.entry(mist_path.to_path_buf()).or_insert(0);
            *version += 1;
            let current_version = *version;

            if map.contains_key(&transpiled.rust_path) {
                let _ = ra
                    .did_change(rust_uri, &transpiled.rust_content, current_version)
                    .await;
            } else {
                let _ = ra.did_open(rust_uri, &transpiled.rust_content).await;
            }

            map.insert(transpiled.rust_path, transpiled.mapping);
        }
    }

    async fn rebuild_module_tree(&self) {
        let ws = match &*self.workspace_folder.lock().await {
            Some(p) => p.clone(),
            None => return,
        };
        let src_root = ws.join("src");
        let package_mist = read_mist_package(&ws);

        // Collect sources first to avoid holding locks during transpile
        let sources: Vec<(PathBuf, String, String)> = {
            let docs = self.documents.lock().await;
            if docs.is_empty() {
                return;
            }
            let mod_decls = compute_mod_decls(&docs, &src_root, &package_mist);
            mod_decls
                .into_iter()
                .filter(|(_, decl)| !decl.is_empty())
                .filter_map(|(path, decl)| docs.get(&path).map(|r| (path, r.to_string(), decl)))
                .collect()
        };

        for (mist_path, source, decl) in sources {
            self.handle_transpile_and_notify(&mist_path, &source, &decl)
                .await;
        }
    }

    async fn publish_diagnostics(&self, uri: Url, diagnostics: Vec<Diagnostic>) {
        let key = uri.clone();
        let prev = self.previous_diagnostics.lock().await.get(&key).cloned();

        if prev.as_ref() != Some(&diagnostics) {
            self.previous_diagnostics
                .lock()
                .await
                .insert(key, diagnostics.clone());
            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(
        &self,
        params: InitializeParams,
    ) -> tower_lsp::jsonrpc::Result<InitializeResult> {
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
            resolve_provider: Some(false),
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
        res.capabilities.hover_provider = Some(HoverProviderCapability::Simple(true));

        let folder_path = params
            .workspace_folders
            .as_ref()
            .and_then(|folders| folders.first())
            .and_then(|folder| folder.uri.to_file_path().ok());

        if let Some(ref path) = folder_path {
            *self.workspace_folder.lock().await = Some(path.clone());
        }

        let ws = self.workspace_folder.clone();
        let ra = self.rust_analyzer.clone();
        let mapping = self.mapping.clone();
        let documents = self.documents.clone();
        let client = self.client.clone();
        let previous_diagnostics = self.previous_diagnostics.clone();
        let notification_rx = self.notification_rx.clone();

        tokio::spawn(async move {
            if let Some(root) = &*ws.lock().await {
                let src_root = root.join("src");

                if let Err(e) = ra.lock().await.initialize(root).await {
                    eprintln!("Failed to initialize rust-analyzer: {e}");
                    return;
                }

                if let Err(e) = ra.lock().await.initialized().await {
                    eprintln!("rust-analyzer initialized failed: {e}");
                    return;
                }

                let mut files = Vec::new();
                collect_mist_files(&src_root, &mut files);

                for file in &files {
                    if let Ok(text) = std::fs::read_to_string(file) {
                        documents
                            .lock()
                            .await
                            .insert(file.clone(), Rope::from_str(&text));
                    }
                }

                eprintln!("Loaded {} mist files", files.len());

                let mapping_c = mapping.clone();
                let documents_c = documents.clone();
                let ra_c = ra.clone();

                for file in &files {
                    if let Some(source) = documents_c.lock().await.get(file).map(|r| r.to_string())
                    {
                        let transpiled = match transpile_mist(file, &source, "") {
                            Ok(t) => t,
                            Err(e) => {
                                eprintln!("transpile error for {:?}: {e}", file);
                                continue;
                            }
                        };

                        if let Some(rust_uri) = clean_lsp_url(&transpiled.rust_path) {
                            let _ = ra_c
                                .lock()
                                .await
                                .did_open(rust_uri, &transpiled.rust_content)
                                .await;
                            mapping_c
                                .lock()
                                .await
                                .insert(transpiled.rust_path, transpiled.mapping);
                        }
                    }
                }

                if let Some(rx) = notification_rx.lock().await.take() {
                    tokio::spawn(handle_ra_notifications(
                        rx,
                        client,
                        mapping,
                        documents,
                        previous_diagnostics,
                    ));
                }

                eprintln!("Ready to use");
            }
        });

        Ok(res)
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let mist_path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => {
                return;
            }
        };
        let source = params.text_document.text;

        self.documents
            .lock()
            .await
            .insert(mist_path.clone(), Rope::from_str(&source));

        self.handle_transpile_and_notify(&mist_path, &source, "")
            .await;
        self.rebuild_module_tree().await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let mist_path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => {
                return;
            }
        };

        if let Some(change) = params.content_changes.into_iter().last() {
            let source = change.text;

            self.documents
                .lock()
                .await
                .insert(mist_path.clone(), Rope::from_str(&source));

            self.handle_transpile_and_notify(&mist_path, &source, "")
                .await;
            self.rebuild_module_tree().await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        let mist_path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => {
                return;
            }
        };

        if let Some(text) = params.text {
            self.documents
                .lock()
                .await
                .insert(mist_path.clone(), Rope::from_str(&text));

            self.handle_transpile_and_notify(&mist_path, &text, "")
                .await;
            self.rebuild_module_tree().await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        let mist_path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => {
                return;
            }
        };

        self.documents.lock().await.remove(&mist_path);

        let rust_path = mist_to_rust_path(&mist_path);
        self.mapping.lock().await.remove(&rust_path);

        if let Some(rust_uri) = clean_lsp_url(&rust_path) {
            let _ = self.rust_analyzer.lock().await.did_close(rust_uri).await;
        }

        self.publish_diagnostics(uri, Vec::new()).await;
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CompletionResponse>> {
        let mist_uri = params.text_document_position.text_document.uri.clone();
        let mist_pos = params.text_document_position.position;

        let mist_path = match mist_uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };
        let source = match self.documents.lock().await.get(&mist_path) {
            Some(r) => r.to_string(),
            None => return Ok(None),
        };

        let mod_decl = self.compute_mod_decl_for_file(&mist_path).await;
        let Some((rust_uri, rust_pos)) = self
            .resolve_mist_via_marker(
                &mist_path,
                &source,
                mist_pos.line,
                mist_pos.character,
                &mod_decl,
            )
            .await
        else {
            return Ok(None);
        };

        let comp_params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: rust_uri },
                position: rust_pos,
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
            context: params.context,
        };

        match self
            .rust_analyzer
            .lock()
            .await
            .request::<lsp_types::request::Completion>(comp_params)
            .await
        {
            Ok(Some(CompletionResponse::Array(items))) => {
                let cleaned: Vec<CompletionItem> =
                    items.into_iter().map(clean_completion_item).collect();
                Ok(Some(CompletionResponse::Array(cleaned)))
            }
            Ok(Some(CompletionResponse::List(list))) => {
                let cleaned: Vec<CompletionItem> =
                    list.items.into_iter().map(clean_completion_item).collect();
                Ok(Some(CompletionResponse::List(CompletionList {
                    is_incomplete: list.is_incomplete,
                    items: cleaned,
                })))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                eprintln!("completion error: {e}");
                Ok(None)
            }
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let mist_uri = params.text_document_position_params.text_document.uri;
        let mist_pos = params.text_document_position_params.position;

        let mist_path = match mist_uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };
        let source = match self.documents.lock().await.get(&mist_path) {
            Some(r) => r.to_string(),
            None => return Ok(None),
        };

        let mod_decl = self.compute_mod_decl_for_file(&mist_path).await;
        let Some((rust_uri, rust_pos)) = self
            .resolve_mist_via_marker(
                &mist_path,
                &source,
                mist_pos.line,
                mist_pos.character,
                &mod_decl,
            )
            .await
        else {
            return Ok(None);
        };

        let gd_params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: rust_uri },
                position: rust_pos,
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
        };

        match self
            .rust_analyzer
            .lock()
            .await
            .request::<lsp_types::request::GotoDefinition>(gd_params)
            .await
        {
            Ok(Some(GotoDefinitionResponse::Scalar(loc))) => {
                let mapped = self.map_rust_to_mist_pos(&loc.uri, &loc.range.start).await;
                match mapped {
                    Some((mist_uri, mist_start)) => {
                        let mist_range = Range {
                            start: mist_start,
                            end: Position {
                                line: mist_start.line,
                                character: mist_start.character + 1,
                            },
                        };
                        Ok(Some(GotoDefinitionResponse::Scalar(Location {
                            uri: mist_uri,
                            range: mist_range,
                        })))
                    }
                    None => Ok(Some(GotoDefinitionResponse::Scalar(loc))),
                }
            }
            Ok(Some(GotoDefinitionResponse::Array(locs))) => {
                let mut mapped = Vec::new();
                for loc in locs {
                    if let Some((mist_uri, mist_start)) =
                        self.map_rust_to_mist_pos(&loc.uri, &loc.range.start).await
                    {
                        mapped.push(Location {
                            uri: mist_uri,
                            range: Range {
                                start: mist_start,
                                end: Position {
                                    line: mist_start.line,
                                    character: mist_start.character + 1,
                                },
                            },
                        });
                    }
                }
                Ok(Some(GotoDefinitionResponse::Array(mapped)))
            }
            Ok(Some(GotoDefinitionResponse::Link(links))) => {
                let mut mapped = Vec::new();
                for link in links {
                    if let Some((mist_uri, mist_start)) = self
                        .map_rust_to_mist_pos(&link.target_uri, &link.target_selection_range.start)
                        .await
                    {
                        mapped.push(LocationLink {
                            origin_selection_range: link.origin_selection_range,
                            target_uri: mist_uri,
                            target_range: Range {
                                start: mist_start,
                                end: Position {
                                    line: mist_start.line,
                                    character: mist_start.character + 1,
                                },
                            },
                            target_selection_range: Range {
                                start: mist_start,
                                end: Position {
                                    line: mist_start.line,
                                    character: mist_start.character + 1,
                                },
                            },
                        });
                    }
                }
                Ok(Some(GotoDefinitionResponse::Link(mapped)))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                eprintln!("goto_definition error: {e}");
                Ok(None)
            }
        }
    }

    async fn hover(&self, params: HoverParams) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
        let mist_uri = params.text_document_position_params.text_document.uri;
        let mist_pos = params.text_document_position_params.position;

        let mist_path = match mist_uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };
        let source = match self.documents.lock().await.get(&mist_path) {
            Some(r) => r.to_string(),
            None => return Ok(None),
        };

        let mod_decl = self.compute_mod_decl_for_file(&mist_path).await;
        let Some((rust_uri, rust_pos)) = self
            .resolve_mist_via_marker(
                &mist_path,
                &source,
                mist_pos.line,
                mist_pos.character,
                &mod_decl,
            )
            .await
        else {
            return Ok(None);
        };

        let h_params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: rust_uri },
                position: rust_pos,
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
        };

        match self
            .rust_analyzer
            .lock()
            .await
            .request::<lsp_types::request::HoverRequest>(h_params)
            .await
        {
            Ok(hover) => Ok(hover),
            Err(e) => {
                eprintln!("hover error: {e}");
                Ok(None)
            }
        }
    }
}

async fn handle_ra_notifications(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<Value>,
    client: Client,
    mapping: Arc<Mutex<HashMap<PathBuf, Mapping>>>,
    _documents: Arc<Mutex<HashMap<PathBuf, Rope>>>,
    previous_diagnostics: Arc<Mutex<HashMap<Url, Vec<Diagnostic>>>>,
) {
    #[derive(Deserialize)]
    struct PublishDiagnosticsParams {
        uri: Url,
        diagnostics: Vec<Diagnostic>,
    }

    while let Some(notification) = rx.recv().await {
        let method = notification
            .get("method")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        match method.as_deref() {
            Some("textDocument/publishDiagnostics") => {
                if let Ok(params) = serde_json::from_value::<PublishDiagnosticsParams>(
                    notification["params"].clone(),
                ) {
                    let rust_uri = params.uri;
                    let diagnostics = params.diagnostics;

                    let mist_uri = rust_uri_to_mist_uri(&rust_uri);

                    let mapped_diagnostics: Vec<Diagnostic> = if let Some(ref _mist_uri) = mist_uri
                    {
                        let rust_path = rust_uri.to_file_path().ok();
                        let map_data =
                            rust_path.and_then(|p| mapping.blocking_lock().get(&p).cloned());

                        diagnostics
                            .into_iter()
                            .filter_map(|diag| {
                                let map = map_data.clone()?;

                                let rust_start = lsp_pos_to_rust_map(&diag.range.start);
                                let rust_end = lsp_pos_to_rust_map(&diag.range.end);

                                let (_, mist_start) = map.find(&rust_start)?;
                                let (_, mist_end) = map.find(&rust_end)?;

                                let mist_start_pos = mist_map_to_lsp_pos(&mist_start);
                                let mist_end_pos = mist_map_to_lsp_pos(&mist_end);

                                let final_end = Position {
                                    line: mist_end_pos.line.max(mist_start_pos.line),
                                    character: if mist_end_pos.line == mist_start_pos.line {
                                        mist_end_pos.character.max(mist_start_pos.character + 1)
                                    } else {
                                        mist_end_pos.character.max(1)
                                    },
                                };

                                Some(Diagnostic {
                                    range: Range {
                                        start: mist_start_pos,
                                        end: final_end,
                                    },
                                    severity: diag.severity,
                                    code: diag.code,
                                    code_description: diag.code_description,
                                    source: diag.source,
                                    message: diag.message,
                                    related_information: diag.related_information,
                                    tags: diag.tags,
                                    data: diag.data,
                                })
                            })
                            .collect()
                    } else {
                        diagnostics
                    };

                    if let Some(mist_uri) = mist_uri.or(Some(rust_uri.clone())) {
                        let prev = previous_diagnostics.lock().await.get(&mist_uri).cloned();

                        if prev.as_ref() != Some(&mapped_diagnostics) {
                            previous_diagnostics
                                .lock()
                                .await
                                .insert(mist_uri.clone(), mapped_diagnostics.clone());
                            client
                                .publish_diagnostics(mist_uri, mapped_diagnostics, None)
                                .await;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

#[tokio::main]
pub async fn start() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        Backend {
            client,
            workspace_folder: Arc::new(Mutex::new(None)),
            previous_diagnostics: Arc::new(Mutex::new(HashMap::new())),
            mapping: Arc::new(Mutex::new(HashMap::new())),
            documents: Arc::new(Mutex::new(HashMap::new())),
            doc_versions: Arc::new(Mutex::new(HashMap::new())),
            rust_analyzer: Arc::new(Mutex::new(
                RustAnalyzer::new(tx).expect("Failed to create rust analyzer"),
            )),
            notification_rx: Arc::new(Mutex::new(Some(rx))),
        }
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

pub fn from_mist_to_rust(path: PathBuf) -> PathBuf {
    mist_to_rust_path(&path)
}

pub fn from_rust_to_mist(path: PathBuf) -> PathBuf {
    rust_to_mist_path(&path)
}

fn read_mist_package(workspace_root: &Path) -> String {
    let toml_path = workspace_root.join("Mist.toml");
    let content = match std::fs::read_to_string(&toml_path) {
        Ok(c) => c,
        Err(_) => return "main.mist".to_string(),
    };
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("package") {
            if let Some(eq_pos) = rest.find('=') {
                let val = rest[eq_pos + 1..]
                    .trim()
                    .trim_matches('"')
                    .trim()
                    .to_string();
                if !val.is_empty() {
                    return val;
                }
            }
        }
    }
    "main.mist".to_string()
}

fn compute_mod_decls(
    documents: &HashMap<PathBuf, Rope>,
    src_root: &Path,
    package_mist: &str,
) -> HashMap<PathBuf, String> {
    let package_path = src_root.join(package_mist);
    let mut result = HashMap::<PathBuf, String>::new();

    // Group .mist files by parent directory
    let mut dirs: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for mist_path in documents.keys() {
        if let Ok(rel) = mist_path.strip_prefix(src_root) {
            let parent = rel.parent().unwrap_or(Path::new(""));
            dirs.entry(src_root.join(parent))
                .or_default()
                .push(mist_path.clone());
        }
    }

    for (dir, files) in &dirs {
        let mut mod_decl = String::new();
        let mut sorted = files.clone();
        sorted.sort();
        for file in &sorted {
            let stem = match file.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s,
                None => continue,
            };
            // Skip the root package file itself
            if *file == package_path {
                continue;
            }
            mod_decl.push_str(&format!("pub mod {};\n", stem));
        }

        if dir == src_root {
            // Root directory declarations go to the package entry file
            result.insert(package_path.clone(), mod_decl);
        } else {
            // Subdirectory: declarations go to a package.mist file if it exists
            let sub_module = dir.join("package.mist");
            if documents.contains_key(&sub_module) {
                result.entry(sub_module).or_default().push_str(&mod_decl);
            }
        }
    }

    result
}

fn clean_completion_item(mut item: CompletionItem) -> CompletionItem {
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
