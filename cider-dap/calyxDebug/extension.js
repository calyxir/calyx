const vscode = require('vscode');
const cp = require('child_process');

// Create an output channel
let outputChannel = vscode.window.createOutputChannel("cider dap");

// CiderDebugAdapter class for managing debug adapter process
class CiderDebugAdapter {
  constructor(adapterPath, cwd, outputChannel) {
    this.adapterPath = adapterPath;
    this.cwd = cwd;
    this.outputChannel = outputChannel;
    this.adapterProcess = null;
  }

  // Start the debug adapter process
  start() {
    logToPanel('Debugger starting...');

    // Spawn a new child process for the debug adapter
    this.adapterProcess = cp.spawn(this.adapterPath, {
      cwd: this.cwd
    });

    // Attach event listener to capture standard output of the adapter process and log it to the output channel
    this.adapterProcess.stdout.on('data', (data) => {
      logToPanel(data.toString());
    });

    // Attach event listener to capture standard error of the adapter process and log it to the output channel
    this.adapterProcess.stderr.on('data', (data) => {
      logToPanel(data.toString());
    });
    logToPanel('Debugger started!');
  }

  // Stop the debug adapter process
  stop() {
    if (this.adapterProcess) {
      // Terminate the adapter process and set it to null
      this.adapterProcess.kill();
      this.adapterProcess = null;
      logToPanel('Debugger stopped.');
    } else {
      logToPanel('No running debug adapter to stop.');
    }
  }
}

// Start debugging
function startDebugging() {
  if (!debugAdapter) {
    // Set the path to the debug adapter and current working directory
    const adapterPath = '/home/basantkhalil/calyx2/target/debug/cider-dap';
    const cwd = vscode.workspace.rootPath;

    // Create a new instance of the CiderDebugAdapter
    debugAdapter = new CiderDebugAdapter(adapterPath, cwd, outputChannel);
  }

  // Start the debug adapter 
  debugAdapter.start();
}

// Stop debugging
function stopDebugging() {
  if (debugAdapter) {
    // Stop the running debug adapter process
    debugAdapter.stop();
  } else {
    logToPanel('No running debug adapter to stop.');
  }
}

// Variable to hold the debug adapter instance
let debugAdapter = null;

function logToPanel(message) {
  console.log("inside logPanel");
  outputChannel.appendLine(message);
}

// Activate the extension
function activate(context) {
  logToPanel('Hello, your extension is now activated!');

  // Register the 'extension.startDebugging' command
  let disposableStart = vscode.commands.registerCommand('cider.startDebugging', startDebugging);
  console.log("after startDebugging");
  context.subscriptions.push(disposableStart);

  // Register the 'extension.stopDebugging' command
  let disposableStop = vscode.commands.registerCommand('cider.stopDebugging', stopDebugging);
  context.subscriptions.push(disposableStop);

  // Register the debug adapter descriptor factory
  vscode.debug.registerDebugAdapterDescriptorFactory('cider-dap', {
    createDebugAdapterDescriptor: (_session) => {
      return new vscode.DebugAdapterExecutable('./cider-dap');
    }
  });
}
function deactivate() {
  logToPanel("deactivate");
}
// Export the activate function to be used as the entry point for the extension
module.exports = {
  activate,
  deactivate
};
