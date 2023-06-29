'use strict';
import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext) {
  // Register a debug configuration provider for Calyx files
  const provider = new CalyxDebugConfigurationProvider();
  context.subscriptions.push(vscode.debug.registerDebugConfigurationProvider('calyx', provider));

  // Register a debug adapter descriptor factory for Calyx files
  context.subscriptions.push(vscode.debug.registerDebugAdapterDescriptorFactory('calyx', new CalyxDebugAdapterDescriptorFactory()));
}

export function deactivate() {
  // Cleanup tasks when the extension is deactivated
}

class CalyxDebugConfigurationProvider implements vscode.DebugConfigurationProvider {
  provideDebugConfigurations(folder: vscode.WorkspaceFolder | undefined, token?: vscode.CancellationToken): vscode.ProviderResult<vscode.DebugConfiguration[]> {
    // Return an array of debug configurations for Calyx files
    return [
      {
        name: 'Debug Calyx',
        type: 'calyx',
        request: 'launch',
        program: '${file}',
        cwd: '${workspaceFolder}'
      }
    ];
  }
}

class CalyxDebugAdapterDescriptorFactory implements vscode.DebugAdapterDescriptorFactory {
  createDebugAdapterDescriptor(session: vscode.DebugSession, executable: vscode.DebugAdapterExecutable | undefined): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
    // Start a new debug adapter server for the Calyx debug adapter
    return new vscode.DebugAdapterServer(0);
  }
}
