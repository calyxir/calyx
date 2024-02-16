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
use document::{ComponentSig, Document};
use goto_definition::DefinitionProvider;
use query_result::QueryResult2;
use serde::Deserialize;
use tower_lsp::lsp_types as lspt;
use tower_lsp::{jsonrpc, Client, LanguageServer, LspService, Server};
use tree_sitter as ts;

use crate::completion::CompletionProvider;
use crate::log::Debug;

extern "C" {
    fn tree_sitter_calyx() -> ts::Language;
}

#[derive(Debug, Deserialize, Default)]
struct Config {
    #[serde(rename = "calyx-lsp")]
    calyx_lsp: CalyxLspConfig,
}

#[derive(Debug, Deserialize)]
struct CalyxLspConfig {
    #[serde(rename = "library-paths")]
    library_paths: Vec<String>,
}

impl Default for CalyxLspConfig {
    fn default() -> Self {
        Self {
            library_paths: vec!["~/.calyx".to_string()],
        }
    }
}

struct Backend {
    client: Client,
    open_docs: RwLock<HashMap<lspt::Url, document::Document>>,
    config: RwLock<Config>,
    /// A map from each open file, to the components defined in that file
    symbols: RwLock<HashMap<lspt::Url, HashMap<String, ComponentSig>>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            open_docs: RwLock::new(HashMap::default()),
            config: RwLock::new(Config::default()),
            symbols: RwLock::new(HashMap::default()),
        }
    }

    fn open(&self, uri: lspt::Url, text: String) {
        let mut map = self.open_docs.write().unwrap();
        map.insert(uri.clone(), Document::new_with_text(uri, &text));
    }

    fn open_path(&self, uri: lspt::Url) {
        fs::read_to_string(uri.to_file_path().unwrap())
            .ok()
            .map(|text| self.open(uri.clone(), text));
    }

    fn exists(&self, uri: &lspt::Url) -> bool {
        let map = self.open_docs.read().unwrap();
        map.contains_key(uri)
    }

    fn read_document<F, T>(&self, uri: &lspt::Url, reader: F) -> Option<T>
    where
        F: FnMut(&Document) -> Option<T>,
    {
        let map = self.open_docs.read().unwrap();
        map.get(uri).and_then(reader)
    }

    fn read_and_open<F, T>(&self, uri: &lspt::Url, reader: F) -> Option<T>
    where
        F: FnMut(&Document) -> Option<T>,
    {
        // if the file doesnt exist, read it's contents and create a doc for it
        if !self.exists(&uri) {
            self.open_path(uri.clone());
            self.update_symbols(&uri);
        }

        self.read_document(&uri, reader)
    }

    fn update<F>(&self, uri: &lspt::Url, updater: F)
    where
        F: FnMut(&mut Document) -> (),
    {
        let mut map = self.open_docs.write().unwrap();
        map.get_mut(uri).map(updater);
    }

    fn update_symbols(&self, url: &lspt::Url) {
        self.symbols
            .write()
            .unwrap()
            .entry(url.clone())
            .and_modify(|map| {
                self.read_document(url, |doc| {
                    for (name, sig) in doc.signatures() {
                        map.insert(name, sig);
                    }
                    Some(())
                });
            })
            .or_insert_with(|| {
                self.read_document(url, |doc| Some(doc.signatures().collect()))
                    .unwrap()
            });
    }

    async fn publish_diagnostics(&self, url: &lspt::Url) {
        let lib_path: PathBuf = self.config.read().unwrap().calyx_lsp.library_paths[0]
            .to_string()
            .into();
        let diags = self
            .read_document(url, |doc| {
                Some(
                    Diagnostic::did_save(&url.to_file_path().unwrap(), &lib_path)
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
            .unwrap_or(vec![]);
        self.client
            .publish_diagnostics(url.clone(), diags, None)
            .await;
    }
}

/// TODO: turn this into a trait
fn newline_split(data: &str) -> Vec<String> {
    let mut res = vec![];
    let mut curr_string = String::new();
    for c in data.chars() {
        if c == '\n' {
            res.push(curr_string);
            curr_string = String::new();
        } else {
            curr_string.push(c);
        }
    }
    res.push(curr_string);
    res
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(
        &self,
        _ip: lspt::InitializeParams,
    ) -> jsonrpc::Result<lspt::InitializeResult> {
        Debug::init("init");
        assert_eq!(newline_split("\n").len(), 2);
        Ok(lspt::InitializeResult {
            server_info: None,
            capabilities: lspt::ServerCapabilities {
                // TODO: switch to incremental parsing
                text_document_sync: Some(lspt::TextDocumentSyncCapability::Options(
                    lspt::TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(lspt::TextDocumentSyncKind::FULL),
                        will_save: None,
                        will_save_wait_until: None,
                        save: Some(lspt::TextDocumentSyncSaveOptions::Supported(true)),
                    },
                )),
                definition_provider: Some(lspt::OneOf::Left(true)),
                completion_provider: Some(lspt::CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), "[".to_string()]),
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                    completion_item: None,
                }),
                hover_provider: Some(lspt::HoverProviderCapability::Simple(false)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _ip: lspt::InitializedParams) {
        self.client
            .log_message(lspt::MessageType::INFO, "server initialized!")
            .await;
    }

    async fn did_open(&self, params: lspt::DidOpenTextDocumentParams) {
        self.open(params.text_document.uri.clone(), params.text_document.text);
        self.publish_diagnostics(&params.text_document.uri).await;
    }

    async fn did_change_configuration(&self, params: lspt::DidChangeConfigurationParams) {
        let config: Config = serde_json::from_value(params.settings).unwrap();
        *self.config.write().unwrap() = config;

        // update the diagnostics on all open documents
        let open_docs: Vec<_> = self.open_docs.read().unwrap().keys().cloned().collect();
        for x in open_docs {
            self.publish_diagnostics(&x).await;
        }
    }

    async fn did_change(&self, params: lspt::DidChangeTextDocumentParams) {
        self.update(&params.text_document.uri, |doc| {
            for event in &params.content_changes {
                doc.parse_whole_text(&event.text);
            }
        });
        self.update_symbols(&params.text_document.uri);
    }

    #[cfg(feature = "diagnostics")]
    async fn did_save(&self, params: lspt::DidSaveTextDocumentParams) {
        let url = &params.text_document.uri;
        self.publish_diagnostics(url).await;
    }

    async fn goto_definition(
        &self,
        params: lspt::GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<lspt::GotoDefinitionResponse>> {
        let url = &params.text_document_position_params.text_document.uri;
        let config = &self.config.read().unwrap();
        Ok(self
            .read_document(url, |doc| {
                doc.thing_at_point(params.text_document_position_params.position.into())
                    .and_then(|thing| doc.find_thing(config, url.clone(), thing))
            })
            .and_then(|gdr| {
                gdr.resolve(|gdr, path| {
                    let url = lspt::Url::from_file_path(path).unwrap();
                    self.read_and_open(&url, |doc| gdr.resume(config, doc))
                })
            })
            .map(|loc| lspt::GotoDefinitionResponse::Scalar(loc)))
    }

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
                            self.read_and_open(&url, |doc| res.resume(&config, doc))
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
