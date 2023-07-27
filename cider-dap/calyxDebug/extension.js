const vscode = require('vscode');
const cp = require('child_process');
const { config } = require('process');

// Create output channel
let outputChannel = vscode.window.createOutputChannel("cider dap");

// Class for debug adapter process
class CiderDebugAdapter {
  constructor(adapterPath, cwd, outputChannel) {
    this.adapterPath = adapterPath;
    this.cwd = cwd;
    this.outputChannel = outputChannel;
    this.adapterProcess = null;
  }
  // Start the debug adapter process
  async start() {
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
    this.adapterProcess = cp.spawn(this.adapterPath, [programName], { cwd: this.cwd });

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


  // Stop debug adapter process
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
async function startDebugging() {
  if (!debugAdapter) {
    // Set the path to the debug adapter and current working directory
    const adapterPath = '/home/basantkhalil/calyx2/target/debug/cider-dap';
    const cwd = vscode.workspace.rootPath;

    // Get the program name from the user
    const program = await getProgramName();
    if (!program) {
      logToPanel('No program selected. Aborting debugging.');
      return;
    }

    // Create an instance of the CiderDebugAdapter
    debugAdapter = new CiderDebugAdapter(adapterPath, cwd, outputChannel);

    // Start the debug adapter with the selected program
    debugAdapter.start();
  }
}

// Stop debugging
function stopDebugging() {
  if (debugAdapter) {
    // Stop the running debug adapter 
    debugAdapter.stop();
  } else {
    logToPanel('No running debug adapter to stop.');
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
    // Return the complete path to the file
    return vscode.workspace.workspaceFolders[0].uri + "/" + fileName;
  } else {
    // Return null if the user canceled the input
    return null;
  }
}

/* // Register the DebugConfigurationProvider
let provider = vscode.debug.registerDebugConfigurationProvider('cider-dap', {
  provideDebugConfigurations: (folder) => {
    return [
      {
        type: 'cider-dap',
        request: 'launch',
        name: 'Ask for file name',
        program: '${command:AskForProgramName}',
        stopOnEntry: true
      }
    ];
  },
  resolveDebugConfiguration: (folder, config) => {
    // Resolve program attribute using the 'getProgramName' command
    return getProgramName(config).then(program => {
      if (program) {
        config.program = program;
      }
      return config;
    });
  }
}); */

// Activate the extension
function activate(context) {
  logToPanel("started activatation");

  let disposableStart = vscode.commands.registerCommand('cider.startDebugging', startDebugging);
  context.subscriptions.push(disposableStart);

  let disposableStop = vscode.commands.registerCommand('cider.stopDebugging', stopDebugging);
  context.subscriptions.push(disposableStop);
  /* 
    // Dispose the provider when the extension is deactivated
    context.subscriptions.push(provider); */

  logToPanel('Hello, your extension is now activated!');
}


function deactivate() {
  logToPanel("deactivate");
}

module.exports = {
  activate,
  deactivate
};
