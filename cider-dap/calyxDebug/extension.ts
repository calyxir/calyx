import * as vscode from "vscode";
import cp = require("child_process");
import net = require("net");

// Hold the debug adapter instance
let debugAdapter = null;
let programName = null; // Store the program name
// Create output channel
let outputChannel = vscode.window.createOutputChannel("Cider dap");

function logToPanel(message) {
  console.log("inside logPanel");
  outputChannel.appendLine(message);
}

// Function to get the program name from the user
async function getProgramName() {
  const fileName = await vscode.window.showInputBox({
    placeHolder:
      "Please enter the name of a futil file in the workspace folder",
    value: "default.futil",
  });

  if (fileName) {
    if (!fileName.startsWith("/")) {
      const path = require("path");
      return path.join(
        vscode.workspace.workspaceFolders[0].uri.fsPath,
        fileName
      );
    }
    return fileName;
  } else {
    return null;
  }
}

// Factory for multi-session
class CiderDebugAdapterDescriptorFactoryServer {
  adapter: CiderDebugAdapter;
  adapterPath: string;
  stdPath: string;
  workspace: string;
  outputChannel: object;

  constructor(adapterPath, stdPath, workspace, outputChannel) {
    logToPanel("inside constructor");
    this.adapter = new CiderDebugAdapter(adapterPath, stdPath, workspace, outputChannel);
    this.stdPath = stdPath;
    this.adapterPath = adapterPath;
    this.workspace = workspace;
    this.outputChannel = outputChannel;
  }

  createDebugAdapterDescriptor(session) {
    // Return a new debug adapter descriptor
    logToPanel("creating adapter descriptor");

    return new vscode.DebugAdapterServer(this._startDebugServer(session));
  }

  _startDebugServer(session) {
    logToPanel("start of startDebugServer");
    // default port: 8888
    const port = vscode.workspace.getConfiguration("cider-dap").port;
    if (!this.adapter.isServerRunning()) {
      logToPanel("server is not running");
      this.adapter.start(port);
      logToPanel("started dap-server");
    }

    logToPanel("exiting startDebugging");
    return port;
  }
}
class CiderDebugAdapter {
  adapterPath: string;
  stdPath: string;
  outputChannel: object;
  cwd: string;
  adapterProcess: cp.ChildProcessWithoutNullStreams | null;
  isRunning: boolean;

  constructor(adapterPath, stdPath, cwd, outputChannel) {
    logToPanel("inside CiderDebugAdapter");
    this.adapterPath = adapterPath;
    this.stdPath = stdPath;
    this.cwd = cwd;
    this.outputChannel = outputChannel;
    this.adapterProcess = null;
    logToPanel("at the end of ciderDebugAdapter");
  }
  isServerRunning() {
    logToPanel("checking if server is running");
    return this.adapterProcess != null && this.adapterProcess.exitCode == null;
  }
  // Start the debug adapter process
  start(port) {
    logToPanel("beginning of start");

    // Spawn a new child process for the debug adapter
    // Include the port as a command line argument
    this.adapterProcess = cp.spawn(
      this.adapterPath,
      ["--port", port, "--tcp", "-l", this.stdPath],
      { cwd: this.cwd }
    );

    // Attach event listener to capture standard output of the adapter process and log it to the output channel
    this.adapterProcess.stdout.on("data", (data) => {
      logToPanel(data.toString());
    });

    // Attach event listener to capture standard error of the adapter process and log it to the output channel
    this.adapterProcess.stderr.on("data", (data) => {
      logToPanel(data.toString());
    });

    this.adapterProcess.on("spawn", () => {
      logToPanel("Debugger started on port " + port + "!");
    });
  }

  stop() {
    if (this.adapterProcess) {
      this.adapterProcess.kill();
      this.adapterProcess = null;
      this.isRunning = false;
      logToPanel("Debugger stopped.");
    } else {
      logToPanel("No running debug adapter to stop.");
    }
  }
}

// Factory for single-session
class CiderDebugAdapterDescriptorFactoryExecutable {
  createDebugAdapterDescriptor(_session) {
    // Use the DebugAdapterExecutable as the debug adapter descriptor
    console.log("inside adapter factory");
    console.log(vscode.workspace.getConfiguration("cider-dap").path);

    return new vscode.DebugAdapterExecutable(
      vscode.workspace.getConfiguration("cider-dap").path,
      [],
      { cwd: vscode.workspace.rootPath }
    );
  }
}

function activate(context) {
  logToPanel("Extension activated!");

  let factory: vscode.DebugAdapterDescriptorFactory;

  // Get session type (multi or single) from package.json configuration

  logToPanel("setting up with configuration '" + vscode.workspace.getConfiguration("cider-dap").sessionType + "'. You will need to reload after changing the settings if a different mode is desired.")

  switch (vscode.workspace.getConfiguration("cider-dap").sessionType) {
    case "Multi-Session":
      factory = new CiderDebugAdapterDescriptorFactoryServer(
        vscode.workspace.getConfiguration("cider-dap").path,
        vscode.workspace.getConfiguration("cider-dap").std_lib,
        vscode.workspace.rootPath,
        outputChannel
      );
      break;

    case "Single-Session":
    default:
      factory = new CiderDebugAdapterDescriptorFactoryExecutable();
      break;
  }

  context.subscriptions.push(
    vscode.debug.registerDebugAdapterDescriptorFactory("cider-dap", factory)
  );
  logToPanel("after start server");

  // Update the adapter path with the serverPort and use it for starting the debug adapter
  logToPanel("before startDebugging");
  logToPanel("Hello, your extension is now activated!");
}

function stopDebugging() {
  if (debugAdapter) {
    debugAdapter.stop();
  } else {
    logToPanel("No running debug adapter to stop.");
  }
}

function deactivate() {
  logToPanel("deactivate");
}

module.exports = {
  activate,
  deactivate,
};
