# Language Server

## Installing

`cargo build --all` will build the language server. Installing it is as easy as putting the binary somewhere on the path. My preferred way to do this is to create a symlink to the binary.

```bash
cd ~/.local/bin
ln -s $CALYX_REPO/target/debug/calyx-lsp calyx-lsp
```

Editor LSP clients will know how to find and use this binary.

## Developing the Language Server

The [language server protocol](language-server-protocol-docs) is general communication protocol defined by Microsoft so that external programs can provide intelligent language features to a variety of editors.

The Calyx language server is built on top of [tower-lsp](), a library that makes it straightforward to define various LSP endpoints. The core of the implemention is an implementation of the `LanguageServer` trait.

### Server Initialization

The `initialize` method is called when the server is first started, and defines what the server supports. Currently, we support:

- `definition_provider`: This handles jump to definition.
- `completion_provider`: Handles completing at point.

Here we also define how we want the client to send us changes when the document changes. Currently, we use `TextDocumentSyncKind::Full` which specifies that the client should resent the entire text document, everytime it changes. This simplifies the implementation, at the cost of efficiency. At some point, we should change this to support incremental updates.

### Server Configuration

The Calyx language-server supports two methods of configuration: the newer "server-pull" model of configuraton where the server requests specific configuration keys from the client, and the `workspace/didChangeConfiguration` method where the client notifies the server of any configuration changes.

The only configuration that the server supports is specifying where to find Calyx libraries. Details for how to specify this configuration is found [here](./editor-highlighting.md).

### Architecture

The `Backend` struct stores the server configuration, and all of the open documents. This is the struct that we implement `LanguageServer` for. There is a lock around the map holding the documents, and a lock around the config.

For certain things, like jumping to definitions out of file, and finding completions for cells defined out of file, computations that start from one document, need to search through other documents. This is handled through the `QueryResult` trait in `query_result.rs`. This is documented in more detail in the source code, but provides a mechanism for searching through multiple documents.

### Tree-sitter Parsing

We use [`tree-sitter`](tree-sitter) to maintain a parse tree of open documents. We could theorectically use the Calyx parser itself for this, but `tree-sitter` provides incremental and error-tolerant parsing and a powerful query language that make it convenient to use.

The grammar is defined in `calyx-lsp/tree-sitter-calyx` and is automatically built when `calyx-lsp` is built.

### Debugging the Server

Most clients launch the server in subprocess which makes it annoying to see the `stdout` of the server process. If you build the server with the `log` feature enabled, the server will log messages to `/tmp/calyx-lsp-debug.log` and will write the tree-sitter parse tree to `/tmp/calyx-lsp-debug-tree.log`. Use the `log::stdout!()` and `log::update!()` macros to write to these files.

[tower-lsp]: https://docs.rs/tower-lsp/latest/tower_lsp/
[language-server-protocol-docs]:  https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition
[tree-sitter]:
