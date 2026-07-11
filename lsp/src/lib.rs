mod code_actions;
mod code_lens;
mod diagnostics;
mod document;
mod semantic;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use compiler::{CompileOptions, CompileStage, Compiler, NYRA_VERSION};
use errors::NyraError;
use nyra_analysis::{
    hover_at, signature_help_at, span_to_lsp_range, DocumentAnalysis, SymbolKind as NyraSymbolKind,
    WorkspaceIndex,
};
use nyra_fmt;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::code_actions::code_actions_for_document;
use crate::code_lens::{collect_test_lenses, test_lens_command};
use crate::diagnostics::diagnostic_from_error;
use crate::document::{apply_changes, full_document_range};
use crate::semantic::{encode_semantic_tokens, semantic_tokens_legend};

const DIAG_DEBOUNCE_MS: u64 = 250;

#[derive(Default)]
struct DocumentState {
    text: String,
    version: i32,
    diag_generation: u64,
}

pub struct NyraLanguageServer {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, DocumentState>>>,
    workspace_roots: Arc<RwLock<Vec<PathBuf>>>,
}

impl NyraLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            workspace_roots: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn file_path(uri: &Url) -> String {
        uri.to_file_path()
            .ok()
            .and_then(|p| p.to_str().map(str::to_string))
            .unwrap_or_else(|| "untitled.ny".into())
    }

    fn canonical_path(path: &str) -> String {
        WorkspaceIndex::canonical_file_key(path)
    }

    async fn workspace_index(&self) -> Option<WorkspaceIndex> {
        let docs = self.documents.read().await;
        let roots = self.workspace_roots.read().await;

        let entry = if let Some(root) = roots.first() {
            resolve::paths::find_main_entry(root).or_else(|| {
                docs.keys().find_map(|uri| {
                    let p = PathBuf::from(Self::file_path(uri));
                    if p.exists() { Some(p) } else { None }
                })
            })
        } else {
            docs.keys().find_map(|uri| {
                let p = PathBuf::from(Self::file_path(uri));
                p.exists().then_some(p)
            })
        }?;

        let mut ws = WorkspaceIndex::from_file(&entry).ok()?;
        for (uri, doc) in docs.iter() {
            let key = Self::canonical_path(&Self::file_path(uri));
            ws.files.insert(key, doc.text.clone());
        }
        Some(WorkspaceIndex::from_sources(ws.root, ws.files))
    }

    async fn schedule_diagnostics(&self, uri: Url) {
        let generation = {
            let mut docs = self.documents.write().await;
            let Some(doc) = docs.get_mut(&uri) else {
                return;
            };
            doc.diag_generation = doc.diag_generation.saturating_add(1);
            doc.diag_generation
        };

        let client = self.client.clone();
        let documents = Arc::clone(&self.documents);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(DIAG_DEBOUNCE_MS)).await;
            let (text, current_gen) = {
                let docs = documents.read().await;
                let Some(doc) = docs.get(&uri) else {
                    return;
                };
                (doc.text.clone(), doc.diag_generation)
            };
            if current_gen != generation {
                return;
            }
            let diags = compile_diagnostics(&Self::file_path(&uri), &text);
            client.publish_diagnostics(uri, diags, None).await;
        });
    }

    async fn publish_diagnostics_immediate(&self, uri: &Url, source: &str) {
        let path = Self::file_path(uri);
        let diags = compile_diagnostics(&path, source);
        self.client
            .publish_diagnostics(uri.clone(), diags, None)
            .await;
    }

    async fn refresh_all_open_diagnostics(&self) {
        let snapshots: Vec<(Url, String)> = {
            let docs = self.documents.read().await;
            docs.iter()
                .map(|(uri, doc)| (uri.clone(), doc.text.clone()))
                .collect()
        };
        for (uri, text) in snapshots {
            self.publish_diagnostics_immediate(&uri, &text).await;
        }
    }
}

fn compile_diagnostics(path: &str, source: &str) -> Vec<Diagnostic> {
    let options = CompileOptions {
        stop_after: Some(CompileStage::Borrow),
        ..CompileOptions::default()
    };
    let Ok(out) = Compiler::compile_source(source, path, &options) else {
        return vec![];
    };
    out.lexer_errors
        .iter()
        .chain(&out.parser_errors)
        .chain(&out.type_errors)
        .chain(&out.borrow_errors)
        .map(|e| diagnostic_from_error(e, DiagnosticSeverity::ERROR))
        .chain(
            out.warnings
                .iter()
                .map(|w| diagnostic_from_error(w, DiagnosticSeverity::WARNING)),
        )
        .collect()
}

fn compile_errors(path: &str, source: &str) -> Vec<NyraError> {
    let options = CompileOptions {
        stop_after: Some(CompileStage::Borrow),
        ..CompileOptions::default()
    };
    let Ok(out) = Compiler::compile_source(source, path, &options) else {
        return vec![];
    };
    out.lexer_errors
        .into_iter()
        .chain(out.parser_errors)
        .chain(out.type_errors)
        .chain(out.borrow_errors)
        .collect()
}


fn parse_doc_program(source: &str, path: &str) -> Option<(ast::Program, Vec<NyraError>)> {
    let (tokens, lex_errs) = lexer::Lexer::new(source, path).tokenize();
    if !lex_errs.is_empty() {
        return None;
    }
    Some(parser::Parser::new(tokens).parse())
}

#[tower_lsp::async_trait]
impl LanguageServer for NyraLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(folders) = params.workspace_folders {
            let mut roots = self.workspace_roots.write().await;
            roots.clear();
            for folder in folders {
                if let Ok(path) = folder.uri.to_file_path() {
                    roots.push(path);
                }
            }
        } else if let Some(root_uri) = params.root_uri {
            if let Ok(path) = root_uri.to_file_path() {
                self.workspace_roots.write().await.push(path);
            }
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        ..Default::default()
                    },
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".into(), ":".into()]),
                    resolve_provider: Some(false),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
                    SemanticTokensOptions {
                        legend: semantic_tokens_legend(),
                        full: Some(SemanticTokensFullOptions::Bool(true)),
                        ..Default::default()
                    },
                )),
                inlay_hint_provider: Some(OneOf::Right(
                    InlayHintServerCapabilities::Options(InlayHintOptions {
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                        resolve_provider: Some(false),
                    }),
                )),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".into(), ",".into()]),
                    ..Default::default()
                }),
                code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
                    code_action_kinds: Some(vec![
                        CodeActionKind::QUICKFIX,
                        CodeActionKind::SOURCE_FIX_ALL,
                        CodeActionKind::SOURCE,
                    ]),
                    resolve_provider: Some(false),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["nyra.runTest".into()],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                document_highlight_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "nyra".into(),
                version: Some(NYRA_VERSION.into()),
            }),
        })
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;
        {
            let mut docs = self.documents.write().await;
            docs.insert(
                uri.clone(),
                DocumentState {
                    text: text.clone(),
                    version,
                    diag_generation: 0,
                },
            );
        }
        self.publish_diagnostics_immediate(&uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let changes = params.content_changes;
        {
            let mut docs = self.documents.write().await;
            if let Some(doc) = docs.get_mut(&uri) {
                apply_changes(&mut doc.text, &changes);
                doc.version = version;
            }
        }
        self.schedule_diagnostics(uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = {
            let docs = self.documents.read().await;
            docs.get(&uri).map(|d| d.text.clone())
        };
        if let Some(text) = text {
            self.publish_diagnostics_immediate(&uri, &text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.write().await.remove(&uri);
        self.client
            .publish_diagnostics(uri, vec![], None)
            .await;
    }

    async fn did_change_watched_files(&self, _params: DidChangeWatchedFilesParams) {
        self.refresh_all_open_diagnostics().await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };
        let path = Self::file_path(&uri);
        let prefix = word_prefix_at(&doc.text, pos.line, pos.character);
        let analysis = DocumentAnalysis::analyze(&doc.text, &path);
        let items: Vec<CompletionItem> = analysis
            .completions(&prefix)
            .into_iter()
            .map(|label| {
                let kind = analysis
                    .symbols
                    .iter()
                    .find(|s| s.name == label)
                    .map(|s| symbol_kind_to_lsp(s.kind))
                    .unwrap_or(CompletionItemKind::TEXT);
                let (insert_text, insert_format) = completion_snippet(&label);
                CompletionItem {
                    label: label.clone(),
                    kind: Some(kind),
                    insert_text,
                    insert_text_format: insert_format,
                    ..Default::default()
                }
            })
            .collect();
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };
        let path = Self::file_path(&uri);
        let Some(content) = hover_at(&doc.text, &path, pos.line, pos.character) else {
            return Ok(None);
        };
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: content,
            }),
            range: None,
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let Some(ws) = self.workspace_index().await else {
            return Ok(None);
        };
        let file = Self::canonical_path(&Self::file_path(&uri));
        let Some(def) = ws.goto_definition_at(&file, pos.line, pos.character) else {
            return Ok(None);
        };
        let target_uri = Url::from_file_path(&def.file).ok();
        let Some(target_uri) = target_uri else {
            return Ok(None);
        };
        let (sl, sc, el, ec) = span_to_lsp_range(&def.span);
        let range = if def.span.start.line > 0 {
            Range {
                start: Position { line: sl, character: sc },
                end: Position { line: el, character: ec },
            }
        } else {
            word_range_at(&ws.files[&def.file], &def.name)
        };
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri: target_uri,
            range,
        })))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let Some(ws) = self.workspace_index().await else {
            return Ok(None);
        };
        let file = Self::canonical_path(&Self::file_path(&uri));
        let refs = ws.find_references_at(&file, pos.line, pos.character);
        let mut out = Vec::new();
        for loc in refs {
            let Ok(target_uri) = Url::from_file_path(&loc.file) else {
                continue;
            };
            let (sl, sc, el, ec) = span_to_lsp_range(&loc.span);
            out.push(Location {
                uri: target_uri,
                range: Range {
                    start: Position { line: sl, character: sc },
                    end: Position { line: el, character: ec },
                },
            });
        }
        Ok(Some(out))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let new_name = params.new_name;
        let Some(ws) = self.workspace_index().await else {
            return Ok(None);
        };
        let file = Self::canonical_path(&Self::file_path(&uri));
        let edits_map = ws.workspace_rename_edits(&file, pos.line, pos.character, &new_name);
        if edits_map.is_empty() {
            return Ok(None);
        }
        let mut changes = HashMap::new();
        for (path, edits) in edits_map {
            let Ok(u) = Url::from_file_path(&path) else {
                continue;
            };
            let text_edits: Vec<TextEdit> = edits
                .into_iter()
                .map(|e| {
                    let (sl, sc, el, ec) = span_to_lsp_range(&e.span);
                    TextEdit {
                        range: Range {
                            start: Position { line: sl, character: sc },
                            end: Position { line: el, character: ec },
                        },
                        new_text: e.new_text,
                    }
                })
                .collect();
            if !text_edits.is_empty() {
                changes.insert(u, text_edits);
            }
        }
        if changes.is_empty() {
            return Ok(None);
        }
        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };
        let path = Self::file_path(&uri);
        let analysis = DocumentAnalysis::analyze(&doc.text, &path);
        Ok(Some(encode_semantic_tokens(&doc.text, &analysis)))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };
        let path = Self::file_path(&uri);
        let analysis = DocumentAnalysis::analyze(&doc.text, &path);
        let hints: Vec<InlayHint> = analysis
            .inlay_hints
            .iter()
            .map(|h| {
                let (kind, padding_left, padding_right) = match h.kind {
                    nyra_analysis::InlayHintKind::Type => {
                        (InlayHintKind::TYPE, None, Some(true))
                    }
                    nyra_analysis::InlayHintKind::Parameter => {
                        (InlayHintKind::PARAMETER, None, Some(true))
                    }
                };
                InlayHint {
                    position: Position {
                        line: h.line,
                        character: h.character,
                    },
                    label: InlayHintLabel::String(h.label.clone()),
                    kind: Some(kind),
                    text_edits: None,
                    tooltip: None,
                    padding_left,
                    padding_right,
                    data: None,
                }
            })
            .collect();
        Ok(Some(hints))
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> Result<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };
        let path = Self::file_path(&uri);
        let analysis = DocumentAnalysis::analyze(&doc.text, &path);
        let Some(info) = signature_help_at(&doc.text, &analysis, pos.line, pos.character) else {
            return Ok(None);
        };
        let params: Vec<ParameterInformation> = info
            .parameters
            .into_iter()
            .map(|label| ParameterInformation {
                label: ParameterLabel::Simple(label),
                documentation: None,
            })
            .collect();
        Ok(Some(SignatureHelp {
            signatures: vec![SignatureInformation {
                label: info.label,
                documentation: info.documentation.map(|d| Documentation::String(d)),
                parameters: Some(params),
                active_parameter: Some(info.active_parameter as u32),
            }],
            active_signature: Some(0),
            active_parameter: Some(info.active_parameter as u32),
        }))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };
        let path = Self::file_path(&uri);
        let errors = compile_errors(&path, &doc.text);
        let only = params
            .context
            .only
            .as_ref()
            .map(|v| v.as_slice());
        let actions = code_actions_for_document(
            &uri,
            &doc.text,
            &path,
            &errors,
            Some(params.range),
            only,
        );
        if actions.is_empty() {
            return Ok(None);
        }
        Ok(Some(actions))
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let uri = params.text_document.uri;
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };
        let path = Self::file_path(&uri);
        let Some((program, errs)) = parse_doc_program(&doc.text, &path) else {
            return Ok(None);
        };
        if !errs.is_empty() {
            return Ok(None);
        }
        let lenses = collect_test_lenses(&program, &path);
        if lenses.is_empty() {
            return Ok(None);
        }
        let out: Vec<CodeLens> = lenses
            .iter()
            .map(|l| CodeLens {
                range: l.range,
                command: Some(test_lens_command(l)),
                data: None,
            })
            .collect();
        Ok(Some(out))
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let file = Self::canonical_path(&Self::file_path(&uri));
        let Some(ws) = self.workspace_index().await else {
            return Ok(None);
        };
        let refs = ws.find_references_at(&file, pos.line, pos.character);
        let mut out = Vec::new();
        for loc in refs {
            if Self::canonical_path(&loc.file) != file && loc.file != file {
                continue;
            }
            let (sl, sc, el, ec) = span_to_lsp_range(&loc.span);
            out.push(DocumentHighlight {
                range: Range {
                    start: Position { line: sl, character: sc },
                    end: Position { line: el, character: ec },
                },
                kind: Some(DocumentHighlightKind::READ),
            });
        }
        Ok(if out.is_empty() { None } else { Some(out) })
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let query = params.query.to_lowercase();
        let Some(ws) = self.workspace_index().await else {
            return Ok(None);
        };
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for loc in &ws.locations {
            if !loc.is_definition {
                continue;
            }
            if !query.is_empty() && !loc.name.to_lowercase().contains(&query) {
                continue;
            }
            let key = format!("{}:{}", loc.file, loc.name);
            if !seen.insert(key) {
                continue;
            }
            let Ok(uri) = Url::from_file_path(&loc.file) else {
                continue;
            };
            let (sl, sc, el, ec) = span_to_lsp_range(&loc.span);
            let range = if loc.span.start.line > 0 {
                Range {
                    start: Position { line: sl, character: sc },
                    end: Position { line: el, character: ec },
                }
            } else {
                Range::default()
            };
            out.push(SymbolInformation {
                name: loc.name.clone(),
                kind: symbol_kind_to_document(loc.kind),
                tags: None,
                deprecated: None,
                location: Location { uri, range },
                container_name: None,
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(if out.is_empty() { None } else { Some(out) })
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };
        let path = Self::file_path(&uri);
        let analysis = DocumentAnalysis::analyze(&doc.text, &path);
        let symbols: Vec<DocumentSymbol> = analysis
            .symbols
            .iter()
            .filter(|s| {
                matches!(
                    s.kind,
                    NyraSymbolKind::Function
                        | NyraSymbolKind::Struct
                        | NyraSymbolKind::Enum
                        | NyraSymbolKind::Constant
                ) && s.span.start.line > 0
            })
            .map(|s| {
                let (sl, sc, el, ec) = span_to_lsp_range(&s.span);
                #[allow(deprecated)]
                DocumentSymbol {
                    name: s.name.clone(),
                    detail: s.detail.clone(),
                    kind: symbol_kind_to_document(s.kind),
                    range: Range {
                        start: Position { line: sl, character: sc },
                        end: Position { line: el, character: ec },
                    },
                    selection_range: Range {
                        start: Position { line: sl, character: sc },
                        end: Position {
                            line: sl,
                            character: sc + s.name.len() as u32,
                        },
                    },
                    children: None,
                    tags: None,
                    deprecated: None,
                }
            })
            .collect();
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };
        let path = Self::file_path(&uri);
        let formatted = nyra_fmt::format_source_or_fallback(&doc.text, &path);
        Ok(Some(vec![TextEdit {
            range: full_document_range(&doc.text),
            new_text: formatted,
        }]))
    }
}

fn completion_snippet(label: &str) -> (Option<String>, Option<InsertTextFormat>) {
    let snippet = match label {
        "fn" => "fn ${1:name}(${2:args}) {\n\t${0}\n}",
        "async" => "async fn ${1:name}(${2:args}) {\n\t${0}\n}",
        "struct" => "struct ${1:Name} {\n\t${2:field}: ${3:string}\n}",
        "test" => "test fn ${1:name}() {\n\t${0}\n}",
        "trait" => "trait ${1:Name} {\n\tfn ${2:method}(self) -> ${3:i32}\n}",
        "impl" => "impl ${1:Trait} for ${2:Type} {\n\t${0}\n}",
        "spawn" => "spawn {\n\t${0}\n}",
        "import" => "import \"${1:stdlib/module.ny}\"",
        _ => return (None, None),
    };
    (
        Some(snippet.to_string()),
        Some(InsertTextFormat::SNIPPET),
    )
}

fn word_range_at(source: &str, name: &str) -> Range {
    for (i, line) in source.lines().enumerate() {
        if let Some(col) = line.find(name) {
            return Range {
                start: Position {
                    line: i as u32,
                    character: col as u32,
                },
                end: Position {
                    line: i as u32,
                    character: (col + name.len()) as u32,
                },
            };
        }
    }
    Range::default()
}

fn word_prefix_at(source: &str, line: u32, character: u32) -> String {
    let line_text = source.lines().nth(line as usize).unwrap_or("");
    let safe_col = (character as usize).min(line_text.len());
    line_text[..safe_col]
        .rsplit(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
        .to_string()
}

fn symbol_kind_to_lsp(kind: NyraSymbolKind) -> CompletionItemKind {
    match kind {
        NyraSymbolKind::Function | NyraSymbolKind::Method | NyraSymbolKind::Extern => {
            CompletionItemKind::FUNCTION
        }
        NyraSymbolKind::Parameter => CompletionItemKind::VARIABLE,
        NyraSymbolKind::Variable | NyraSymbolKind::Constant => CompletionItemKind::VARIABLE,
        NyraSymbolKind::Struct => CompletionItemKind::STRUCT,
        NyraSymbolKind::Enum => CompletionItemKind::ENUM,
        NyraSymbolKind::Field => CompletionItemKind::FIELD,
        NyraSymbolKind::Module => CompletionItemKind::MODULE,
        NyraSymbolKind::Keyword => CompletionItemKind::KEYWORD,
    }
}

fn symbol_kind_to_document(kind: NyraSymbolKind) -> SymbolKind {
    match kind {
        NyraSymbolKind::Function | NyraSymbolKind::Method | NyraSymbolKind::Extern => {
            SymbolKind::FUNCTION
        }
        NyraSymbolKind::Struct => SymbolKind::STRUCT,
        NyraSymbolKind::Enum => SymbolKind::ENUM,
        NyraSymbolKind::Constant | NyraSymbolKind::Variable | NyraSymbolKind::Parameter => {
            SymbolKind::VARIABLE
        }
        _ => SymbolKind::STRING,
    }
}

pub async fn run_stdio() {
    let (service, socket) = LspService::new(NyraLanguageServer::new);
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_diagnostics_finds_type_error() {
        let src = "fn main() { let x: i32 = \"text\" }";
        let diags = compile_diagnostics("main.ny", src);
        assert!(!diags.is_empty());
        assert!(diags[0].message.contains("type") || diags[0].message.contains("mismatch"));
    }
}
