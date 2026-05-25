use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower_lsp_server::jsonrpc::Result as LspResult;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{Client, LanguageServer};

use crate::ast::Ast;
use crate::parser::ParserState;
use crate::include_resolver::IncludeResolver;
use crate::config::LspConfig;
use crate::semantic::SemanticCache;
use crate::diagnostics;

const DIAGNOSTIC_DEBOUNCE_MS: u64 = 150;

pub struct Backend {
    pub client: Client,
    pub documents: Arc<Mutex<HashMap<Uri, ParserState>>>,
    pub include_resolver: Arc<Mutex<IncludeResolver>>,
    pub config: Arc<Mutex<LspConfig>>,
    pub debounce_counters: Arc<Mutex<HashMap<Uri, u64>>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(Mutex::new(HashMap::new())),
            include_resolver: Arc::new(Mutex::new(IncludeResolver::new())),
            config: Arc::new(Mutex::new(LspConfig::default())),
            debounce_counters: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl LanguageServer for Backend {
    async fn initialize(
        &self,
        params: InitializeParams,
    ) -> LspResult<InitializeResult> {
        log::info!("initialize: {:?}", params.root_uri);

        // Parse initialization options from Zed extension
        if let Some(init_options) = params.initialization_options {
            if let Ok(config) = serde_json::from_value::<LspConfig>(init_options) {
                *self.config.lock().unwrap() = config;
            }
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        "$".to_string(),
                        "{".to_string(),
                        "_".to_string(),
                        "/".to_string(),
                    ]),
                    resolve_provider: Some(true),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                document_formatting_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "lammps-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            offset_encoding: None,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        log::info!("lammps-lsp initialized");
    }

    async fn shutdown(&self) -> LspResult<()> {
        log::info!("shutdown");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        log::info!("did_open: {:?}", uri);

        let uri_str = uri.to_string();
        let version = params.text_document.version;

        let (diags, uri_clone) = {
            let mut docs = self.documents.lock().unwrap();
            docs.insert(uri.clone(), ParserState::new(&params.text_document.text));

            // Build AST and semantic while holding docs lock
            // (Ast borrows from state.source inside docs)
            let doc_state = docs.get(&uri).unwrap();
            let ast = Ast::new(&doc_state.source, &doc_state.tree);
            let semantic = SemanticCache::build(&ast, &uri_str);

            // Acquire config — safe because order is consistent (docs before config)
            let config = self.config.lock().unwrap();
            let d = diagnostics::run_diagnostics(&ast, &semantic, &config.diagnostics, &uri_str);
            drop(config);
            (d, uri.clone())
        };

        self.client
            .publish_diagnostics(uri_clone, diags, Some(version))
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;

        // Apply edits immediately
        {
            let mut docs = self.documents.lock().unwrap();
            if let Some(state) = docs.get_mut(&uri) {
                for change in params.content_changes {
                    match change.range {
                        Some(range) => {
                            state.apply_edit(
                                range.start.line as usize,
                                range.start.character as usize,
                                range.end.line as usize,
                                range.end.character as usize,
                                &change.text,
                            );
                        }
                        None => {
                            *state = ParserState::new(&change.text);
                        }
                    }
                }
            }
        }

        // Debounced diagnostics: increment counter, sleep, then check if still latest
        let counter = {
            let mut counters = self.debounce_counters.lock().unwrap();
            let entry = counters.entry(uri.clone()).or_insert(0);
            *entry += 1;
            *entry
        };

        let client = self.client.clone();
        let documents = self.documents.clone();
        let config = self.config.clone();
        let debounce_counters = self.debounce_counters.clone();
        let uri_str = uri.to_string();

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(DIAGNOSTIC_DEBOUNCE_MS)).await;

            // Skip if a newer change arrived during the sleep
            let latest = *debounce_counters.lock().unwrap().get(&uri).unwrap_or(&0);
            if latest != counter {
                return;
            }

            let diags = {
                let docs = documents.lock().unwrap();
                let doc_state = match docs.get(&uri) {
                    Some(s) => s,
                    None => return,
                };
                let ast = Ast::new(&doc_state.source, &doc_state.tree);
                let semantic = SemanticCache::build(&ast, &uri_str);
                let config_lock = config.lock().unwrap();
                diagnostics::run_diagnostics(&ast, &semantic, &config_lock.diagnostics, &uri_str)
            };

            client.publish_diagnostics(uri.clone(), diags, Some(version)).await;
        });
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        log::info!("did_close: {:?}", uri);
        self.documents.lock().unwrap().remove(&uri);
        self.debounce_counters.lock().unwrap().remove(&uri);
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        log::info!("did_save: {:?}", uri);

        let uri_str = uri.to_string();

        let (diags, uri_clone) = {
            let docs = self.documents.lock().unwrap();
            let doc_state = match docs.get(&uri) {
                Some(s) => s,
                None => return,
            };
            let ast = Ast::new(&doc_state.source, &doc_state.tree);
            let semantic = SemanticCache::build(&ast, &uri_str);

            let config = self.config.lock().unwrap();
            let d = diagnostics::run_diagnostics(&ast, &semantic, &config.diagnostics, &uri_str);
            drop(config);
            (d, uri.clone())
        };

        self.client
            .publish_diagnostics(uri_clone, diags, None)
            .await;
    }

    // Completion (Phase 4)
    async fn completion(
        &self,
        params: CompletionParams,
    ) -> LspResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;

        let docs = self.documents.lock().unwrap();
        let state = match docs.get(&uri) {
            Some(s) => s,
            None => return Ok(None),
        };

        let ast = crate::ast::Ast::new(&state.source, &state.tree);
        let semantic = crate::semantic::SemanticCache::build(&ast, &uri.to_string());
        Ok(crate::completion::run_completion(&ast, &semantic, position))
    }

    async fn completion_resolve(
        &self,
        item: CompletionItem,
    ) -> LspResult<CompletionItem> {
        // All documentation is already provided in the initial completion
        // response, so resolution is a pass-through.
        Ok(item)
    }

    // Hover
    async fn hover(
        &self,
        params: HoverParams,
    ) -> LspResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri.clone();
        let position = params.text_document_position_params.position;

        let docs = self.documents.lock().unwrap();
        let state = match docs.get(&uri) {
            Some(s) => s,
            None => return Ok(None),
        };

        let ast = crate::ast::Ast::new(&state.source, &state.tree);
        let semantic = crate::semantic::SemanticCache::build(&ast, &uri.to_string());
        Ok(crate::hover::run_hover(&ast, &semantic, position))
    }

    // Go-to-definition
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> LspResult<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri.clone();
        let position = params.text_document_position_params.position;

        let docs = self.documents.lock().unwrap();
        let state = match docs.get(&uri) {
            Some(s) => s,
            None => return Ok(None),
        };

        let ast = crate::ast::Ast::new(&state.source, &state.tree);
        let semantic = crate::semantic::SemanticCache::build(&ast, &uri.to_string());
        Ok(crate::goto::run_goto_definition(
            &ast,
            &semantic,
            position,
            &uri.to_string(),
        ))
    }

    // Find references
    async fn references(
        &self,
        params: ReferenceParams,
    ) -> LspResult<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;

        let docs = self.documents.lock().unwrap();
        let state = match docs.get(&uri) {
            Some(s) => s,
            None => return Ok(None),
        };

        let ast = crate::ast::Ast::new(&state.source, &state.tree);
        let semantic = crate::semantic::SemanticCache::build(&ast, &uri.to_string());
        Ok(crate::references::run_references(
            &ast,
            &semantic,
            position,
            &uri.to_string(),
        ))
    }

    // Document symbols
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> LspResult<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri.clone();

        let docs = self.documents.lock().unwrap();
        let state = match docs.get(&uri) {
            Some(s) => s,
            None => return Ok(None),
        };

        let ast = crate::ast::Ast::new(&state.source, &state.tree);
        let semantic = crate::semantic::SemanticCache::build(&ast, &uri.to_string());
        let symbols = crate::symbols::run_document_symbols(&ast, &semantic);

        if symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(DocumentSymbolResponse::Nested(symbols)))
        }
    }

    // Rename
    async fn rename(
        &self,
        params: RenameParams,
    ) -> LspResult<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;
        let new_name = params.new_name.clone();

        let docs = self.documents.lock().unwrap();
        let state = match docs.get(&uri) {
            Some(s) => s,
            None => return Ok(None),
        };

        let ast = crate::ast::Ast::new(&state.source, &state.tree);
        let semantic = crate::semantic::SemanticCache::build(&ast, &uri.to_string());
        Ok(crate::rename::run_rename(
            &ast,
            &semantic,
            position,
            &new_name,
            &uri.to_string(),
        ))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> LspResult<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri.clone();
        let position = params.position;

        let docs = self.documents.lock().unwrap();
        let state = match docs.get(&uri) {
            Some(s) => s,
            None => return Ok(None),
        };

        let ast = crate::ast::Ast::new(&state.source, &state.tree);
        let semantic = crate::semantic::SemanticCache::build(&ast, &uri.to_string());
        Ok(crate::rename::prepare_rename(
            &ast,
            &semantic,
            position,
        ))
    }

    // Formatting
    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> LspResult<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri.clone();

        let fmt_config = {
            let config = self.config.lock().unwrap();
            config.formatting.clone()
        };

        let docs = self.documents.lock().unwrap();
        let state = match docs.get(&uri) {
            Some(s) => s,
            None => return Ok(None),
        };

        let edits = crate::formatting::run_formatting(&state.source, &fmt_config);
        if edits.is_empty() {
            Ok(None)
        } else {
            Ok(Some(edits))
        }
    }
}
