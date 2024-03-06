# Calyx README

Calyx VSCode Extension that adds basic syntax highlighting and integrates the Calyx LSP server for jump to definition, and autocomplete.

## Installation

You can install the extension from the VSCode store by searching for Calyx. Alternatively, you can link this directory directly into your vscode extensions folder:

```bash
cd $HOME/.vscode/extensions
ln -s <calyx root directory>/tools/vscode calyx.calyx-1.0.0
```

Then reload VSCode.

## LSP Integration

For the LSP integration, you also need `calyx-lsp` installed. You can build it with `cargo build --all` from the Calyx root directory. Then link the executable to your path:

```bash
cd $HOME/.local/bin
ln -s <calyx root directory>/target/debug/calyx-lsp calyx-lsp
```
