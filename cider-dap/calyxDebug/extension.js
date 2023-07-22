const vscode = require('vscode');
const cp = require('child_process');

let outputChannel;

function startDebugging() {
  // After attaching and starting debugging, call the panel printing function
  logToPanel('Debugger attached and debugging started.');
  logToPanel('Some additional information...');
}

function logToPanel(message) {
  // Create an output channel 
  if (!outputChannel) {
    outputChannel = vscode.window.createOutputChannel('Cider DAP');
  }

  // Log the message to the output channel
  outputChannel.appendLine(message);

  // Show the message in the panel as well
  vscode.window.showInformationMessage(message);
}

function activate(context) {
  // Create the outputChannel only once when the extension activates
  outputChannel = vscode.window.createOutputChannel('Cider DAP');

  // Register a command to start debugging
  const disposableStartDebugging = vscode.commands.registerCommand('extension.cider-dap.startDebugging', startDebugging);
  context.subscriptions.push(disposableStartDebugging);

  // Register a command for testing purposes
  const disposableTest = vscode.commands.registerCommand('extension.cider-dap.test', function () {
    // The code you want to run when the command is executed

    // Log to the output channel
    logToPanel('Executing extension.cider-dap.test');

    // Listen for stdout data
    const proc = cp.spawn('your_command_here', ['arg1', 'arg2']); // Replace 'your_command_here' with the actual command to be executed
    proc.stdout.on('data', (data) => {
      logToPanel(`stdout: ${data}`);
    });

    // Listen for stderr data
    proc.stderr.on('data', (data) => {
      logToPanel(`stderr: ${data}`);
    });

    // Listen for close event
    proc.on('close', (code) => {
      logToPanel(`child process exited with code ${code}`);
    });
  });

  context.subscriptions.push(disposableTest);
}

function deactivate() { }

module.exports = {
  activate,
  deactivate
};
