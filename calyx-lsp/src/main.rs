mod completion;
mod convert;
mod diagnostic;
mod document;
mod goto_definition;
mod log;
mod query_result;
mod ts_utils;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

use convert::{Point, Range};
use diagnostic::Diagnostic;
use document::Document;
use goto_definition::DefinitionProvider;
use query_result::QueryResult;
use serde::Deserialize;
use tower_lsp::lsp_types::{self as lspt, Url};
use tower_lsp::{jsonrpc, Client, LanguageServer, LspService, Server};
use tree_sitter as ts;

use crate::completion::CompletionProvider;
use crate::log::Debug;

extern "C" {
    /// Bind the tree-sitter parser to something that we can use in Rust
    fn tree_sitter_calyx() -> ts::Language;
}

#[derive(Debug, Deserialize, Default)]
struct Config {
    #[serde(rename = "calyxLsp")]
    calyx_lsp: CalyxLspConfig,
}

#[derive(Debug, Deserialize)]
struct CalyxLspConfig {
    #[serde(rename = "libraryPaths")]
    library_paths: Vec<String>,
}

impl Default for CalyxLspConfig {
    fn default() -> Self {
        Self {
            library_paths: vec!["~/.calyx".to_string()],
        }
    }
}

/// Data for the Calyx Language Server
struct Backend {
    /// Connection to the client that is used for sending data
    client: Client,
    /// Currently open documents
    open_docs: RwLock<HashMap<lspt::Url, document::Document>>,
    /// Server configuration
    config: RwLock<Config>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            open_docs: RwLock::new(HashMap::default()),
            config: RwLock::new(Config::default()),
        }
    }

    /// Open a new document located at `url` with contents `text`.
    fn open(&self, url: lspt::Url, text: String) {
        let mut map = self.open_docs.write().unwrap();
        map.insert(url.clone(), Document::new_with_text(url, &text));
    }

    /// Open a new document located at `url` with contents read
    /// from that path on the system.
    fn open_path(&self, url: lspt::Url) {
        if let Ok(text) = fs::read_to_string(url.to_file_path().unwrap()) {
            self.open(url.clone(), text)
        }
    }

    /// Check if a path is currently opened.
    fn exists(&self, url: &lspt::Url) -> bool {
        let map = self.open_docs.read().unwrap();
        map.contains_key(url)
    }

    /// Read the contents of `url` using function `reader`.
    fn read_document<F, T>(&self, url: &lspt::Url, reader: F) -> Option<T>
    where
        F: FnMut(&Document) -> Option<T>,
    {
        self.open_docs
            .read()
            .ok()
            .and_then(|map| map.get(url).and_then(reader))
    }

    /// Read the contents of `url` using function `reader`.
    /// If the document isn't already open, then open it first.
    fn read_and_open<F, T>(&self, url: &lspt::Url, reader: F) -> Option<T>
    where
        F: FnMut(&Document) -> Option<T>,
    {
        // if the file doesnt exist, read it's contents and create a doc for it
        if !self.exists(url) {
            self.open_path(url.clone());
        }

        self.read_document(url, reader)
    }

    /// Update the document at `url` using function `updater`.
    fn update<F>(&self, url: &lspt::Url, updater: F)
    where
        F: FnMut(&mut Document),
    {
        self.open_docs
            .write()
            .ok()
            .map(|mut map| map.get_mut(url).map(updater));
    }

    /// Publish diagnostics for document `url`.
    async fn publish_diagnostics(&self, url: &lspt::Url) {
        // TODO: factor the bulk of this method somewhere else
        let lib_path: PathBuf =
            self.config.read().unwrap().calyx_lsp.library_paths[0]
                .to_string()
                .into();
        let diags = self
            .read_document(url, |doc| {
                Some(
                    Diagnostic::did_save(
                        &url.to_file_path().unwrap(),
                        &lib_path,
                    )
                    .into_iter()
                    .filter_map(|diag| {
                        doc.byte_to_point(diag.pos_start).and_then(|s| {
                            doc.byte_to_point(diag.pos_end)
                                .map(|e| (Range::new(s, e), diag.msg))
                        })
                    })
                    .map(|(range, message)| lspt::Diagnostic {
                        range: range.into(),
                        severity: Some(lspt::DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("calyx".to_string()),
                        message,
                        related_information: None,
                        tags: None,
                        data: None,
                    })
                    .inspect(|diag| log::stdout!("{diag:#?}"))
                    .collect(),
                )
            })
            .unwrap_or_default();
        self.client
            .publish_diagnostics(url.clone(), diags, None)
            .await;
    }

    /// Publish diagnostics for every open file.
    async fn publish_all_diagnostics(&self) {
        let open_docs: Vec<_> =
            self.open_docs.read().unwrap().keys().cloned().collect();
        for x in open_docs {
            self.publish_diagnostics(&x).await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    /// LSP method: 'initialize'
    async fn initialize(
        &self,
        _ip: lspt::InitializeParams,
    ) -> jsonrpc::Result<lspt::InitializeResult> {
        Debug::init("init");
        Ok(lspt::InitializeResult {
            server_info: None,
            capabilities: lspt::ServerCapabilities {
                // TODO: switch to incremental parsing
                text_document_sync: Some(
                    lspt::TextDocumentSyncCapability::Options(
                        lspt::TextDocumentSyncOptions {
                            open_close: Some(true),
                            change: Some(lspt::TextDocumentSyncKind::FULL),
                            will_save: None,
                            will_save_wait_until: None,
                            save: Some(
                                lspt::TextDocumentSyncSaveOptions::Supported(
                                    true,
                                ),
                            ),
                        },
                    ),
                ),
                definition_provider: Some(lspt::OneOf::Left(true)),
                completion_provider: Some(lspt::CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        "[".to_string(),
                    ]),
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                    completion_item: None,
                }),
                hover_provider: Some(lspt::HoverProviderCapability::Simple(
                    false,
                )),
                ..Default::default()
            },
        })
    }

    /// LSP method: 'initialized'
    /// Guaranteed to be the first method called after the initialization
    /// process has completed. Here we pull the configuration that we need
    /// from the client.
    async fn initialized(&self, _ip: lspt::InitializedParams) {
        // get library paths option
        let values = self
            .client
            .configuration(vec![lspt::ConfigurationItem {
                scope_uri: Some(Url::parse("file:///libraryPaths").unwrap()),
                section: Some("calyxLsp".to_string()),
            }])
            .await;

        // if we have a value, parse it and update our config
        if let Ok(val) = values {
            let config: CalyxLspConfig =
                serde_json::from_value(val[0].clone()).unwrap();
            self.config.write().unwrap().calyx_lsp = config;
        }

        // force update of diagnostics because the configuration
        // can update the library-paths which might affect which
        // primitives are in scope, thus affecting diagnostics
        // TODO: does this do anything? have any documents been opened yet?
        self.publish_all_diagnostics().await;

        self.client
            .log_message(lspt::MessageType::INFO, "server initialized!")
            .await;
    }

    /// LSP method: 'textDocument/didOpen'
    /// Called when the client opens a new document. We get the entire
    /// text of the document.
    async fn did_open(&self, params: lspt::DidOpenTextDocumentParams) {
        self.open(params.text_document.uri.clone(), params.text_document.text);
        self.publish_diagnostics(&params.text_document.uri).await;
    }

    /// LSP method: 'workspace/didChangeConfiguration'
    /// Called when the client notifies us that some configuration has changed.
    /// It passes us the entire configuration.
    /// Unfortunately, we can't count on this always being called. VSCode for
    /// example, no longer sends this notification.
    async fn did_change_configuration(
        &self,
        params: lspt::DidChangeConfigurationParams,
    ) {
        log::stdout!("document/didConfigurationChange");
        let config: Config = serde_json::from_value(params.settings).unwrap();
        *self.config.write().unwrap() = config;

        // force update of diagnostics because the configuration
        // can update the library-paths which might affect which
        // primitives are in scope, thus affecting diagnostics
        self.publish_all_diagnostics().await;
    }

    /// LSP method: 'textDocument/didChange'
    /// Called when the client updates a text document. Here we process all
    /// the text_update events in the order that they are defined in `params`.
    /// Because we are using the `Full` sync-mode, this should be a single
    /// event containing the entire updated source.
    async fn did_change(&self, params: lspt::DidChangeTextDocumentParams) {
        self.update(&params.text_document.uri, |doc| {
            for event in &params.content_changes {
                doc.parse_whole_text(&event.text);
            }
        });
    }

    /// LSP method: 'textDocument/didSave'
    /// Called when the document was just saved. We use to this update the
    /// diagnostics for the saved file.
    #[cfg(feature = "diagnostics")]
    async fn did_save(&self, params: lspt::DidSaveTextDocumentParams) {
        let url = &params.text_document.uri;
        self.publish_diagnostics(url).await;
    }

    /// LSP method: 'textDocument/definition'
    /// Called when the client requests that we go to a definition.
    async fn goto_definition(
        &self,
        params: lspt::GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<lspt::GotoDefinitionResponse>> {
        let url = &params.text_document_position_params.text_document.uri;
        let config = &self.config.read().unwrap();
        Ok(self
            .read_document(url, |doc| {
                doc.thing_at_point(
                    params.text_document_position_params.position.into(),
                )
                .and_then(|thing| doc.find_thing(config, url.clone(), thing))
            })
            .and_then(|gdr| {
                gdr.resolve(|gdr, path| {
                    let url = lspt::Url::from_file_path(path).unwrap();
                    self.read_and_open(&url, |doc| gdr.resume(config, doc))
                })
            })
            .map(lspt::GotoDefinitionResponse::Scalar))
    }

    /// LSP method: 'textDocument/completion'
    /// Called when the client requests completion for a point in the file.
    async fn completion(
        &self,
        params: lspt::CompletionParams,
    ) -> jsonrpc::Result<Option<lspt::CompletionResponse>> {
        let url = &params.text_document_position.text_document.uri;
        let point: Point = params.text_document_position.position.into();
        let trigger_char = params.context.and_then(|cc| cc.trigger_character);
        let config = self.config.read().unwrap();
        Ok(self
            .read_document(url, |doc| {
                doc.complete(trigger_char.as_deref(), &point, &config)
            })
            .map(|reses| {
                reses
                    .into_iter()
                    .filter_map(|res| {
                        res.resolve(|res, path| {
                            let url = lspt::Url::from_file_path(path).unwrap();
                            self.read_and_open(&url, |doc| {
                                res.resume(&config, doc)
                            })
                        })
                    })
                    .flatten()
                    .collect::<Vec<_>>()
            })
            .map(|completions| {
                lspt::CompletionResponse::Array(
                    completions.into_iter().map(|ci| ci.into()).collect(),
                )
            }))
    }

    /// LSP method: 'shutdown'
    async fn shutdown(&self) -> jsonrpc::Result<()> {
        log::stdout!("shutdown");
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
