// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from 'vscode';

import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
} from 'vscode-languageclient/node';

// This method is called when your extension is activated
// Your extension is activated the very first time the command is executed
export function activate(_context: vscode.ExtensionContext) {

  const serverOptions: ServerOptions = {
    command: 'calyx-lsp',
    args: []
  }

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      // Active functionality on files of these languages.
      {
        language: 'calyx',
      },
    ],
  };

  // start the language client
  const client = new LanguageClient('calyx-lsp', serverOptions, clientOptions);
  client.start();

}

// This method is called when your extension is deactivated
export function deactivate() {}
