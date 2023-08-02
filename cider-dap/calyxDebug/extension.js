const vscode = require('vscode');
const cp = require('child_process');
const { config } = require('process');


// Create output channel
let outputChannel = vscode.window.createOutputChannel("cider dap");


class CiderDebugAdapter {
  constructor(adapterPath, cwd, outputChannel) {
    this.adapterPath = adapterPath;
    this.cwd = cwd;
    this.outputChannel = outputChannel;
    this.adapterProcess = null;
  }


  // Start the debug adapter process
  async start(port) {  // accept the port parameter
    logToPanel('Debugger starting...');
    // Get the program name from the user
    const programName = await getProgramName();

    if (!programName) {
      logToPanel('No program selected. Aborting debugging.');
      return;
    }
    // Verify if the file exists at the provided path
    const fs = require('fs');
    if (!fs.existsSync(programName)) {
      logToPanel(`File not found: ${programName}`);
      return;
    }

    // Spawn a new child process for the debug adapter
    // Include the port as a command line argument
    this.adapterProcess = cp.spawn(this.adapterPath, [programName, '--port', port, "--tcp"], { cwd: this.cwd });

    // Attach event listener to capture standard output of the adapter process and log it to the output channel
    this.adapterProcess.stdout.on('data', (data) => {
      logToPanel(data.toString());
    });

    // Attach event listener to capture standard error of the adapter process and log it to the output channel
    this.adapterProcess.stderr.on('data', (data) => {
      logToPanel(data.toString());
    });

    logToPanel('Debugger started on port ' + port + '!');
  }


  // Stop debug adapter process
  stop() {
    if (this.adapterProcess) {
      this.adapterProcess.kill();
      this.adapterProcess = null;
      this.isRunning = false;
      logToPanel('Debugger stopped.');
    } else {
      logToPanel('No running debug adapter to stop.');
    }
  }
}


// Hold the debug adapter instance
let debugAdapter = null;
let programName = null; // Store the program name


function logToPanel(message) {
  console.log("inside logPanel");
  outputChannel.appendLine(message);
}


// Function to get the program name from the user
async function getProgramName() {
  const fileName = await vscode.window.showInputBox({
    placeHolder: 'Please enter the name of a futil file in the workspace folder',
    value: 'default.futil'
  });

  if (fileName) {
    if (!fileName.startsWith('/')) {
      const path = require('path');
      return path.join(vscode.workspace.workspaceFolders[0].uri.fsPath, fileName);
    }
    return fileName;
  } else {
    return null;
  }
}
// Start debugging
async function startDebugging(arg) {
  logToPanel("inside startDebugging");

  if (!debugAdapter) {
    const adapterPath = '/home/basantkhalil/calyx2/target/debug/cider-dap';
    const cwd = vscode.workspace.rootPath;

    debugAdapter = new CiderDebugAdapter(adapterPath, cwd, outputChannel);
  }
  // Prompt for the port
  const portInput = await vscode.window.showInputBox({
    placeHolder: 'Please enter the port number',
    value: '8888'  // This is the default value
  });

  // If the user entered a value, parse it to an integer
  const port = portInput ? parseInt(portInput, 10) : 1234;
  await debugAdapter.start(port);
  logToPanel("exiting startDebugging");
  return;
}


// Stop debugging
function stopDebugging() {
  if (debugAdapter) {
    debugAdapter.stop();
  } else {
    logToPanel('No running debug adapter to stop.');
  }
}


// Activate the extension
function activate(context) {
  logToPanel("Extension activated!");

  let disposableStart = vscode.commands.registerCommand('cider.startDebugging', startDebugging);
  context.subscriptions.push(disposableStart);

  let disposableStop = vscode.commands.registerCommand('cider.stopDebugging', stopDebugging);
  context.subscriptions.push(disposableStop);

  logToPanel('Hello, your extension is now activated!');
}


function deactivate() {
  logToPanel("deactivate");
}


module.exports = {
  activate,
  deactivate
};



