import * as vscode from 'vscode';
import * as cp from "child_process"

// Hold the debug adapter instance
let debugAdapter = null;
// Create output channel
const outputChannel = vscode.window.createOutputChannel("Cider dap");
const r = new String(vscode.workspace.workspaceFolders[0].uri)
const root = r.substring(r.indexOf("://") + 3).toString()

const disposables: vscode.Disposable[] = []

function logToPanel(message) {
  outputChannel.appendLine(message);
}

class CiderDebugAdapterDescriptorFactoryServer implements vscode.DebugAdapterDescriptorFactory {
  private adapter: CiderDebugAdapter

  createDebugAdapterDescriptor(session: vscode.DebugSession, executable: vscode.DebugAdapterExecutable): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
    let stdPath = vscode.workspace.getConfiguration("cider-dap").std_lib;
    let adapterPath = vscode.workspace.getConfiguration("cider-dap").path //tried just using the executable.command but that doesnt work idk why
    const port = vscode.workspace.getConfiguration("cider-dap").port;

    this.adapter = new CiderDebugAdapter(adapterPath, stdPath, root, outputChannel);

    if (!this.adapter.isServerRunning()) {
      logToPanel("adapter descripter: calling adapter.start(port)")
      let adapterPromise = this.adapter.start(port)
      return adapterPromise.then((res) => {
        return new vscode.DebugAdapterServer(res);
      }, () => { throw "Failed to start debug server" })
    }
    else {
      return new vscode.DebugAdapterServer(port)
    }
  }
  dispose() {
    logToPanel("disposed multi session factory")
    if (this.adapter) {
      this.adapter.stop()
    }
  }
}

// Factory for single-session
class CiderDebugAdapterDescriptorFactoryExecutable implements vscode.DebugAdapterDescriptorFactory {
  createDebugAdapterDescriptor(session: vscode.DebugSession, executable: vscode.DebugAdapterExecutable): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
    // Use the DebugAdapterExecutable as the debug adapter descriptor

    return new vscode.DebugAdapterExecutable(
      vscode.workspace.getConfiguration("cider-dap").path,
      [],
      { cwd: root }
    );
  }
  dispose() {
    logToPanel("disposed single session factory")
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
    this.adapterPath = adapterPath;
    this.stdPath = stdPath;
    this.cwd = cwd;
    this.outputChannel = outputChannel;
    this.adapterProcess = null;
  }
  isServerRunning() {
    return this.adapterProcess != null && this.adapterProcess.exitCode == null;
  }
  // Start the debug adapter process
  start(port) {
    // Spawn a new child process for the debug adapter
    // Include the port as a command line argument
    return new Promise<number>((resolve, reject) => {
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
        setTimeout(() => resolve(port), 700) //short wait to let the thing start running 
      });
      this.adapterProcess.on("error", () => {
        logToPanel("Debugger failed to start");
        reject(-1)
      });
    })
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

function activate(context) {
  logToPanel("Extension activated!");

  let factory: vscode.DebugAdapterDescriptorFactory;

  // Get session type (multi or single) from package.json configuration
  logToPanel("setting up with configuration '" + vscode.workspace.getConfiguration("cider-dap").sessionType + "'. You will need to reload after changing the settings if a different mode is desired.")

  switch (vscode.workspace.getConfiguration("cider-dap").sessionType) {
    case "Multi-Session":
      factory = new CiderDebugAdapterDescriptorFactoryServer();
      break;

    case "Single-Session":
    default:
      factory = new CiderDebugAdapterDescriptorFactoryExecutable();
      break;
  }

  context.subscriptions.push(
    vscode.debug.registerDebugAdapterDescriptorFactory("cider-dap", factory)
  );
  logToPanel("before disposables push")
  disposables.push(vscode.debug.registerDebugAdapterDescriptorFactory("cider-dap", factory))
  // Update the adapter path with the serverPort and use it for starting the debug adapter - ??
  logToPanel("Hello, your extension is now activated! after disposables push");


}

function stopDebugging() {
  if (debugAdapter) {
    debugAdapter.stop();
  } else {
    logToPanel("No running debug adapter to stop.");
  }
}

function deactivate() {
  //kill server 
  // when is this called i don't see it on the output logs
  logToPanel("deactivate");
  disposables.forEach(d => d.dispose())
}

module.exports = {
  activate,
  deactivate,
};
