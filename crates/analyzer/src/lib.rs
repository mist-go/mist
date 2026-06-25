pub mod rust_analyzer;
pub mod transpiler;

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use mist_parser::error::ParseError;
use mist_parser::rev_mapper::{Mapping, MistMap, RustMap};
use ropey::Rope;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::Mutex;
use tower_lsp::lsp_types::{self, *};
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::rust_analyzer::RustAnalyzer;
use crate::transpiler::{TranspileError, transpile_mist, transpile_mist_no_sem};

static MARKER_COUNTER: AtomicU64 = AtomicU64::new(0);

const KEYWORDS: [&'static str; 24] = [
    "if", "else", "for", "while", "match", "return", "break", "continue", "struct", "enum",
    "class", "trait", "impl", "pub", "mut", "let", "true", "false", "dyn", "loop", "unsafe",
    "override", "module", "void ",
];

fn keyword_completion_items() -> impl Iterator<Item = CompletionItem> {
    KEYWORDS.into_iter().map(|kw| CompletionItem {
        label: kw.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        insert_text: Some(kw.to_string()),
        ..Default::default()
    })
}

#[derive(Debug)]
struct Backend {
    client: Client,
    workspace_folder: Arc<Mutex<Option<PathBuf>>>,
    previous_diagnostics: Arc<Mutex<HashMap<Url, Vec<Diagnostic>>>>,
    rust_analyzer: Arc<Mutex<RustAnalyzer>>,
    mapping: Arc<Mutex<HashMap<PathBuf, Mapping>>>,
    documents: Arc<Mutex<HashMap<PathBuf, Rope>>>,
    doc_versions: Arc<Mutex<HashMap<PathBuf, i32>>>,
    last_rust_contents: Arc<Mutex<HashMap<PathBuf, String>>>,
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

/// Inject a marker at the exact cursor position. If the cursor is inside or
/// at the start of an identifier-like token, the entire token is replaced with
/// the marker so parsing doesn't break from a mid-token split.
fn inject_marker_at(source: &str, line: u32, character: u32, marker: &str) -> Option<String> {
    let offset = byte_offset_from_lsp(source, line, character)?;

    let before = &source[..offset];
    let after = &source[offset..];

    // Identifier chars immediately before and after the cursor (as byte-counts).
    let trail_bytes: usize = before
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .map(|c| c.len_utf8())
        .sum();
    let lead_bytes: usize = after
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .map(|c| c.len_utf8())
        .sum();

    if trail_bytes > 0 || lead_bytes > 0 {
        // Cursor touches an identifier — replace the whole token with the marker.
        let token_start = offset - trail_bytes;
        let token_end = offset + lead_bytes;

        let mut s = String::with_capacity(source.len() + marker.len() + 4);
        s.push_str(&source[..token_start]);
        s.push_str(marker);
        s.push_str(&source[token_end..]);
        return Some(s);
    }

    // Not on an identifier.  If the cursor is on a `.` treat it as a field-access
    // and inject the marker as an identifier right after the dot.
    if after.starts_with('.') {
        let dot_end = offset + '.'.len_utf8();
        let mut s = String::with_capacity(source.len() + marker.len() + 4);
        s.push_str(&source[..dot_end]);
        s.push_str(marker);
        s.push_str(&source[dot_end..]);
        return Some(s);
    }

    // Whitespace / other punctuation — inject as standalone expression.
    let mut s = String::with_capacity(source.len() + marker.len() + 4);
    s.push_str(&source[..offset]);
    s.push(' ');
    s.push_str(marker);
    s.push(' ');
    s.push_str(&source[offset..]);
    Some(s)
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

fn byte_offset_to_lsp_pos(source: &str, offset: usize) -> Position {
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    Position {
        line,
        character: col,
    }
}

/// Holds saved state for restoring rust-analyzer content after a temporary
/// marker-driven request (goto, hover, completion). Only populated when
/// markered content was pushed to rust-analyzer and must be reverted.
struct MarkerRestore {
    rust_uri: Url,
    mist_path: PathBuf,
    original_content: String,
    original_version: i32,
}

/// Re-parse the source to extract a proper LSP diagnostic from the error.
/// Returns None when the error is a semantic check failure (can be multiple
/// errors) or when re-parsing unexpectedly succeeds.
fn transpile_error_to_diagnostic(source: &str, error: TranspileError<'_>) -> Vec<Diagnostic> {
    match error {
        TranspileError::Parse(ParseError::PreAst(pest_err)) => {
            let (line, col) = match pest_err.line_col {
                pest::error::LineColLocation::Pos((l, c)) => (l, c),
                pest::error::LineColLocation::Span((l, c), _) => (l, c),
            };
            let message = match &pest_err.variant {
                pest::error::ErrorVariant::ParsingError {
                    positives,
                    negatives,
                } => {
                    let mut msg = String::new();
                    if !positives.is_empty() {
                        msg.push_str("expected ");
                        for (i, r) in positives.iter().enumerate() {
                            if i > 0 {
                                msg.push_str(" or ");
                            }
                            msg.push_str(&format!("{r:?}"));
                        }
                    }
                    if !negatives.is_empty() {
                        if !msg.is_empty() {
                            msg.push_str(", ");
                        }
                        msg.push_str("unexpected ");
                        for (i, r) in negatives.iter().enumerate() {
                            if i > 0 {
                                msg.push_str(" or ");
                            }
                            msg.push_str(&format!("{r:?}"));
                        }
                    }
                    if msg.is_empty() {
                        msg.push_str("parse error");
                    }
                    msg
                }
                pest::error::ErrorVariant::CustomError { message } => message.clone(),
            };
            vec![Diagnostic {
                range: Range {
                    start: Position {
                        line: line as u32 - 1,
                        character: col as u32 - 1,
                    },
                    end: Position {
                        line: line as u32 - 1,
                        character: col as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("mist".to_string()),
                message,
                ..Default::default()
            }]
        }
        TranspileError::Parse(ParseError::Ast(ast_err)) => {
            let start = byte_offset_to_lsp_pos(source, ast_err.span.start());
            let end = byte_offset_to_lsp_pos(source, ast_err.span.end());
            vec![Diagnostic {
                range: Range { start, end },
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("mist".to_string()),
                message: ast_err.error_message.clone(),
                ..Default::default()
            }]
        }
        TranspileError::Semantic(e) => e
            .into_iter()
            .map(|e| Diagnostic {
                range: Range {
                    start: Position {
                        line: e.line as u32 - 1,
                        character: e.column as u32 - 1,
                    },
                    end: Position {
                        line: e.line as u32 - 1,
                        character: e.column as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("mist".to_string()),
                message: e.error_message,
                ..Default::default()
            })
            .collect(),
    }
}

impl Backend {
    /// Resolve a Mist cursor position to a Rust position by injecting a unique marker
    /// into the source at the cursor, transpiling, and finding the marker in the output.
    ///
    /// The third return element is an optional `MarkerRestore` that the caller MUST
    /// pass to `restore_after_marker` **after** it finishes its rust-analyzer request.
    /// This ensures markered content never pollutes the permanent rust-analyzer state.
    async fn resolve_mist_via_marker(
        &self,
        mist_path: &Path,
        source: &str,
        line: u32,
        character: u32,
        extra_mod_decl: &str,
    ) -> Option<(Url, Position, Option<MarkerRestore>)> {
        let id = MARKER_COUNTER.fetch_add(1, Ordering::Relaxed);
        let marker = format!("__mist_mk{id:x}__");

        let modified = match inject_marker_at(source, line, character, &marker) {
            Some(m) => m,
            None => {
                eprintln!("[marker] inject_marker_at returned None for LSP({line},{character})");
                return None;
            }
        };

        let transpiled = match transpile_mist_no_sem(mist_path, &modified, extra_mod_decl) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[marker] transpile failed for {marker} at LSP({line},{character}): {e}");

                // Fallback: the source has a syntax error that prevents transpilation.
                // Use the last known working Rust content + mapping to inject the
                // marker at the nearest valid Rust position.
                let mist_target = MistMap(line as usize + 1, character as usize);
                let mut rust_path = crate::from_mist_to_rust(mist_path.to_path_buf());
                if mist_path.file_name().and_then(|n| n.to_str()) == Some("package.mist") {
                    rust_path.set_file_name("mod.rs");
                }

                let (last_mapping, last_content, original_version) = {
                    let map_guard = self.mapping.lock().await;
                    let content_guard = self.last_rust_contents.lock().await;
                    let versions = self.doc_versions.lock().await;
                    let m = map_guard.get(&rust_path)?.clone();
                    let c = content_guard.get(mist_path)?.clone();
                    let v = versions.get(mist_path).copied().unwrap_or(0);
                    (m, c, v)
                };

                let (_rust_before, mist_before) = last_mapping.find_by_mist(&mist_target)?;

                let rust_next = last_mapping
                    .map
                    .iter()
                    .filter(|(_, mist)| *mist > mist_before)
                    .min_by_key(|(_, mist)| *mist)
                    .map(|(rust, _)| *rust);

                let (modified_rust, rust_pos) = if let Some(next_rust) = rust_next {
                    let lsp_line = (next_rust.0 - 1) as u32;
                    let lsp_col = next_rust.1 as u32;
                    let modified = inject_marker_at(&last_content, lsp_line, lsp_col, &marker)?;
                    let pos = find_marker_position(&modified, &marker)?;
                    (modified, pos)
                } else {
                    let modified = format!("{last_content}\nlet _ = {};", marker);
                    let pos = find_marker_position(&modified, &marker)?;
                    (modified, pos)
                };

                let rust_uri = clean_lsp_url(&rust_path)?;

                // Save original state, then push markered content temporarily.
                {
                    let mut ra = self.rust_analyzer.lock().await;
                    let mut versions = self.doc_versions.lock().await;
                    let temp_version = original_version + 1;
                    versions.insert(mist_path.to_path_buf(), temp_version);
                    let _ = ra
                        .did_change(rust_uri.clone(), &modified_rust, temp_version)
                        .await;
                }

                let restore = MarkerRestore {
                    rust_uri: rust_uri.clone(),
                    mist_path: mist_path.to_path_buf(),
                    original_content: last_content,
                    original_version,
                };

                eprintln!(
                    "[marker] {marker} (fallback): mist LSP({line},{character}) -> rust LSP({},{})",
                    rust_pos.line, rust_pos.character
                );

                return Some((rust_uri, rust_pos, Some(restore)));
            }
        };
        let rust_pos = match find_marker_position(&transpiled.rust_content, &marker) {
            Some(p) => p,
            None => {
                eprintln!("[marker] marker {marker} not found in transpiled output");
                return None;
            }
        };
        let rust_uri = match clean_lsp_url(&transpiled.rust_path) {
            Some(u) => u,
            None => {
                eprintln!(
                    "[marker] failed to create URI for {:?}",
                    transpiled.rust_path
                );
                return None;
            }
        };

        eprintln!(
            "[marker] {marker}: mist LSP({line},{character}) -> rust LSP({},{})",
            rust_pos.line, rust_pos.character
        );

        // If the actual source can't be transpiled, rust-analyzer's file is stale.
        // Push the markered transpiled content temporarily — the caller MUST restore.
        let restore = if transpile_mist(mist_path, source, extra_mod_decl).is_err() {
            let original_content = self
                .last_rust_contents
                .lock()
                .await
                .get(mist_path)
                .cloned()?;
            let original_version = self
                .doc_versions
                .lock()
                .await
                .get(mist_path)
                .copied()
                .unwrap_or(0);

            {
                let mut ra = self.rust_analyzer.lock().await;
                let mut versions = self.doc_versions.lock().await;
                let temp_version = original_version + 1;
                versions.insert(mist_path.to_path_buf(), temp_version);
                let _ = ra
                    .did_change(rust_uri.clone(), &transpiled.rust_content, temp_version)
                    .await;
            }

            Some(MarkerRestore {
                rust_uri: rust_uri.clone(),
                mist_path: mist_path.to_path_buf(),
                original_content,
                original_version,
            })
        } else {
            None
        };

        Some((rust_uri, rust_pos, restore))
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
                eprintln!("transpile error for {:?}: {e:?}", mist_path);
                let diag = transpile_error_to_diagnostic(source, e);
                if let Some(uri) = clean_lsp_url(mist_path) {
                    self.publish_diagnostics(uri, diag).await;
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
            self.last_rust_contents
                .lock()
                .await
                .insert(mist_path.to_path_buf(), transpiled.rust_content);
        }

        // Clear any previous diagnostics — transpile succeeded.
        if let Some(mist_uri) = clean_lsp_url(mist_path) {
            self.publish_diagnostics(mist_uri, vec![]).await;
        }
    }

    async fn rebuild_module_tree(&self) {
        let ws = match &*self.workspace_folder.lock().await {
            Some(p) => p.clone(),
            None => return,
        };
        let src_root = ws.join("src");
        let package_mist = read_mist_package(&ws);

        self.ensure_implicit_packages(&src_root).await;

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

    /// Create synthetic `package.mist` documents for subdirectories that contain
    /// .mist files but no real package.mist (implicit packages).
    async fn ensure_implicit_packages(&self, src_root: &Path) {
        let mut docs = self.documents.lock().await;
        ensure_implicit_packages_impl(&mut *docs, src_root);
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

    /// Restore rust-analyzer content that was temporarily replaced by a marker.
    /// Only restores if no intervening change has bumped the version further.
    async fn restore_after_marker(&self, restore: MarkerRestore) {
        let current_version = self
            .doc_versions
            .lock()
            .await
            .get(&restore.mist_path)
            .copied()
            .unwrap_or(0);

        if current_version != restore.original_version + 1 {
            // Something else changed the content in the meantime — don't
            // clobber it with stale data.
            return;
        }

        let mut ra = self.rust_analyzer.lock().await;
        let mut versions = self.doc_versions.lock().await;
        let new_version = current_version + 1;
        versions.insert(restore.mist_path.clone(), new_version);
        let _ = ra
            .did_change(restore.rust_uri, &restore.original_content, new_version)
            .await;
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
        let last_rust_contents = self.last_rust_contents.clone();
        let client = self.client.clone();
        let previous_diagnostics = self.previous_diagnostics.clone();
        let notification_rx = self.notification_rx.clone();

        tokio::spawn(async move {
            if let Some(root) = &*ws.lock().await {
                let src_root = root.join("src");

                // Hold the lock across both initialize + initialized so the
                // editor's didOpen cannot race in-between and send notifications
                // to rust-analyzer before it has received Initialized.
                {
                    let mut guard = ra.lock().await;
                    if let Err(e) = guard.initialize(root).await {
                        eprintln!("Failed to initialize rust-analyzer: {e}");
                        return;
                    }
                    if let Err(e) = guard.initialized().await {
                        eprintln!("rust-analyzer initialized failed: {e}");
                        return;
                    }
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

                ensure_implicit_packages_impl(&mut *documents.lock().await, &src_root);

                // Compute mod_decls before any transpile so the root file gets its
                // child module declarations from the very first didOpen.
                let package_mist = read_mist_package(root);
                let src_root = root.join("src");
                let mod_decls = {
                    let docs = documents.lock().await;
                    compute_mod_decls(&docs, &src_root, &package_mist)
                };

                for file in &files {
                    let decl = mod_decls.get(file).map(String::as_str).unwrap_or("");
                    let source = match documents.lock().await.get(file) {
                        Some(r) => r.to_string(),
                        None => {
                            eprintln!("missing document for {:?}", file);
                            continue;
                        }
                    };
                    let transpiled = match transpile_mist(file, &source, decl) {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("transpile error for {:?}: {e:?}", file);
                            continue;
                        }
                    };

                    if let Some(rust_uri) = clean_lsp_url(&transpiled.rust_path) {
                        let _ = ra
                            .lock()
                            .await
                            .did_open(rust_uri, &transpiled.rust_content)
                            .await;
                        mapping
                            .lock()
                            .await
                            .insert(transpiled.rust_path, transpiled.mapping);
                        last_rust_contents
                            .lock()
                            .await
                            .insert(file.clone(), transpiled.rust_content);
                    }
                }

                // Also transpile synthetic package.mist files (implicit packages)
                // so rust-analyzer has their content from the start.
                let synthetic_paths: Vec<PathBuf> = documents
                    .lock()
                    .await
                    .keys()
                    .filter(|p| {
                        !files.contains(p)
                            && p.file_name().and_then(|n| n.to_str()) == Some("package.mist")
                    })
                    .cloned()
                    .collect();
                for syn_path in &synthetic_paths {
                    let decl = mod_decls.get(syn_path).map(String::as_str).unwrap_or("");
                    let source = documents
                        .lock()
                        .await
                        .get(syn_path)
                        .map(|r| r.to_string())
                        .unwrap_or_default();
                    if let Ok(transpiled) = transpile_mist(syn_path, &source, decl) {
                        if let Some(rust_uri) = clean_lsp_url(&transpiled.rust_path) {
                            let _ = ra
                                .lock()
                                .await
                                .did_open(rust_uri, &transpiled.rust_content)
                                .await;
                            mapping
                                .lock()
                                .await
                                .insert(transpiled.rust_path, transpiled.mapping);
                            last_rust_contents
                                .lock()
                                .await
                                .insert(syn_path.clone(), transpiled.rust_content);
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
        let Some((rust_uri, rust_pos, marker_restore)) = self
            .resolve_mist_via_marker(
                &mist_path,
                &source,
                mist_pos.line,
                mist_pos.character,
                &mod_decl,
            )
            .await
        else {
            eprintln!(
                "[completion] resolve_mist_via_marker returned None at ({}, {})",
                mist_pos.line, mist_pos.character
            );
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

        let completion_result = self
            .rust_analyzer
            .lock()
            .await
            .request::<lsp_types::request::Completion>(comp_params)
            .await;

        if let Some(restore) = marker_restore {
            self.restore_after_marker(restore).await;
        }

        match completion_result {
            Ok(Some(CompletionResponse::Array(items))) => {
                let mut cleaned: Vec<CompletionItem> =
                    items.into_iter().map(clean_completion_item).collect();

                let existing: HashSet<String> =
                    cleaned.iter().map(|item| item.label.clone()).collect();

                cleaned.extend(
                    keyword_completion_items().filter(|item| !existing.contains(&item.label)),
                );

                if matches!(
                    current_scope(&source, mist_pos.line, mist_pos.character),
                    Scope::Module
                ) {
                    cleaned.push(CompletionItem {
                        label: "function".to_string(),
                        kind: Some(CompletionItemKind::SNIPPET),
                        insert_text: Some("$1 $2($3)\n{\n$4\n}".to_string()),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        sort_text: Some("1".to_string()),
                        ..Default::default()
                    });
                }

                for item in &mut cleaned {
                    if item.label.starts_with("__") {
                        item.sort_text = Some("zzzz".to_string());
                    }
                }

                Ok(Some(CompletionResponse::Array(cleaned)))
            }

            Ok(Some(CompletionResponse::List(mut list))) => {
                let mut cleaned: Vec<CompletionItem> =
                    list.items.into_iter().map(clean_completion_item).collect();

                let existing: HashSet<String> =
                    cleaned.iter().map(|item| item.label.clone()).collect();

                cleaned.extend(
                    keyword_completion_items().filter(|item| !existing.contains(&item.label)),
                );

                if matches!(
                    current_scope(&source, mist_pos.line, mist_pos.character),
                    Scope::Module
                ) {
                    cleaned.push(CompletionItem {
                        label: "function".to_string(),
                        kind: Some(CompletionItemKind::SNIPPET),
                        insert_text: Some("$1 $2($3)\n{\n$4\n}".to_string()),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        sort_text: Some("1".to_string()),
                        ..Default::default()
                    });
                }

                for item in &mut cleaned {
                    if item.label.starts_with("__") {
                        item.sort_text = Some("zzzz".to_string());
                    }
                }

                list.items = cleaned;

                Ok(Some(CompletionResponse::List(list)))
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
        eprintln!(
            "[goto_definition] entered at mist ({}, {})",
            mist_pos.line, mist_pos.character
        );

        let mist_path = match mist_uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };
        let source = match self.documents.lock().await.get(&mist_path) {
            Some(r) => r.to_string(),
            None => return Ok(None),
        };

        let mod_decl = self.compute_mod_decl_for_file(&mist_path).await;
        let Some((rust_uri, rust_pos, marker_restore)) = self
            .resolve_mist_via_marker(
                &mist_path,
                &source,
                mist_pos.line,
                mist_pos.character,
                &mod_decl,
            )
            .await
        else {
            eprintln!(
                "[goto_definition] resolve_mist_via_marker returned None at ({}, {})",
                mist_pos.line, mist_pos.character
            );
            return Ok(None);
        };

        let gd_params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: rust_uri.clone(),
                },
                position: rust_pos,
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
        };
        eprintln!(
            "[goto_definition] rust-analyzer request at {} ({},{})",
            rust_uri, rust_pos.line, rust_pos.character
        );

        let result = self
            .rust_analyzer
            .lock()
            .await
            .request::<lsp_types::request::GotoDefinition>(gd_params)
            .await;

        // Restore original content before processing result — must happen
        // even when the request fails to keep rust-analyzer state clean.
        if let Some(restore) = marker_restore {
            self.restore_after_marker(restore).await;
        }

        match result {
            Ok(Some(GotoDefinitionResponse::Scalar(loc))) => {
                eprintln!("[goto_definition] Scalar response");
                match self.map_rust_to_mist_pos(&loc.uri, &loc.range.start).await {
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
                    None => {
                        if let Some(normalized) =
                            loc.uri.to_file_path().ok().and_then(|p| clean_lsp_url(&p))
                        {
                            eprintln!("[goto_definition] returning raw rust loc");
                            Ok(Some(GotoDefinitionResponse::Scalar(Location {
                                uri: normalized,
                                range: loc.range,
                            })))
                        } else {
                            Ok(Some(GotoDefinitionResponse::Scalar(loc)))
                        }
                    }
                }
            }
            Ok(Some(GotoDefinitionResponse::Array(locs))) => {
                eprintln!("[goto_definition] Array response with {} items", locs.len());
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
                    } else if let Some(normalized) =
                        loc.uri.to_file_path().ok().and_then(|p| clean_lsp_url(&p))
                    {
                        mapped.push(Location {
                            uri: normalized,
                            range: loc.range,
                        });
                    }
                }
                Ok(Some(GotoDefinitionResponse::Array(mapped)))
            }
            Ok(Some(GotoDefinitionResponse::Link(links))) => {
                eprintln!("[goto_definition] Link response with {} items", links.len());
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
                    } else if let Some(normalized) = link
                        .target_uri
                        .to_file_path()
                        .ok()
                        .and_then(|p| clean_lsp_url(&p))
                    {
                        mapped.push(LocationLink {
                            origin_selection_range: link.origin_selection_range,
                            target_uri: normalized,
                            target_range: link.target_range,
                            target_selection_range: link.target_selection_range,
                        });
                    }
                }
                Ok(Some(GotoDefinitionResponse::Link(mapped)))
            }
            Ok(None) => {
                eprintln!("[goto_definition] rust-analyzer returned None");
                Ok(None)
            }
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
        let Some((rust_uri, rust_pos, marker_restore)) = self
            .resolve_mist_via_marker(
                &mist_path,
                &source,
                mist_pos.line,
                mist_pos.character,
                &mod_decl,
            )
            .await
        else {
            eprintln!(
                "[hover] resolve_mist_via_marker returned None at ({}, {})",
                mist_pos.line, mist_pos.character
            );
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

        let result = self
            .rust_analyzer
            .lock()
            .await
            .request::<lsp_types::request::HoverRequest>(h_params)
            .await;

        if let Some(restore) = marker_restore {
            self.restore_after_marker(restore).await;
        }

        match result {
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
                        let map_data = match rust_path {
                            Some(ref p) => mapping.lock().await.get(p).cloned(),
                            None => None,
                        };

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
            last_rust_contents: Arc::new(Mutex::new(HashMap::new())),
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
            // Skip the root package entry file (e.g. main.mist)
            if *file == package_path {
                continue;
            }
            // Skip any subdirectory package.mist — it's the directory's own
            // entry file, not a sibling submodule.
            if stem == "package" {
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

    // For each subdirectory that is a Mist module, add pub mod <dirname>;
    // to the parent directory's declarations so the parent's transpiled Rust
    // output includes the child module declaration.
    for dir in dirs.keys() {
        if dir == src_root {
            continue;
        }
        let dirname = match dir.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        let parent = dir.parent().unwrap();
        let parent_target = if parent == src_root {
            package_path.clone()
        } else {
            let pkg = parent.join("package.mist");
            if !documents.contains_key(&pkg) {
                continue;
            }
            pkg
        };
        result
            .entry(parent_target)
            .or_default()
            .push_str(&format!("pub mod {};\n", dirname));
    }

    result
}

fn ensure_implicit_packages_impl(docs: &mut HashMap<PathBuf, Rope>, src_root: &Path) {
    let mut dirs_with_mist: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for mist_path in docs.keys() {
        if let Ok(rel) = mist_path.strip_prefix(src_root) {
            if let Some(parent) = rel.parent() {
                if !parent.as_os_str().is_empty() {
                    dirs_with_mist
                        .entry(src_root.join(parent))
                        .or_default()
                        .push(mist_path.clone());
                }
            }
        }
    }

    for dir in dirs_with_mist.keys() {
        let pkg_path = dir.join("package.mist");
        if !docs.contains_key(&pkg_path) {
            let dirname = dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("module")
                .to_string();
            let content = format!("pub module {dirname};\n");
            docs.insert(pkg_path, Rope::from_str(&content));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Scope {
    Module,
    Struct,
    Enum,
    Class,
    Trait,
    Impl,
    Block,
    Unknown,
}

fn scope_for_line(line: &str) -> Scope {
    for token in line.split_whitespace() {
        match token {
            "struct" => return Scope::Struct,
            "enum" => return Scope::Enum,
            "class" => return Scope::Class,
            "trait" => return Scope::Trait,
            "impl" => return Scope::Impl,
            _ => {}
        }
    }
    Scope::Block
}

fn current_scope(source: &str, line: u32, character: u32) -> Scope {
    let cursor_offset = match byte_offset_from_lsp(source, line, character) {
        Some(offset) => offset,
        None => return Scope::Unknown,
    };

    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut brace_starts: Vec<usize> = Vec::new();

    for (i, c) in source[..cursor_offset].char_indices() {
        if in_string {
            if c == '"' {
                in_string = false;
            }
            continue;
        }

        match c {
            '{' => {
                depth += 1;
                brace_starts.push(i);
            }
            '}' if depth > 0 => {
                depth -= 1;
                brace_starts.pop();
            }
            '"' => in_string = true,
            _ => {}
        }
    }

    if depth == 0 {
        return Scope::Module;
    }

    let Some(&brace_pos) = brace_starts.last() else {
        return Scope::Block;
    };

    let before = &source[..brace_pos];
    let before_trimmed = before.trim_end();

    let line_start = before_trimmed[..before_trimmed.len()]
        .rfind('\n')
        .map(|pos| pos + 1)
        .unwrap_or(0);

    scope_for_line(&before_trimmed[line_start..])
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
