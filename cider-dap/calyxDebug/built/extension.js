"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var vscode = require("vscode");
var cp = require("child_process");
// Hold the debug adapter instance
var debugAdapter = null;
var programName = null; // Store the program name
// Create output channel
var outputChannel = vscode.window.createOutputChannel("Cider dap");
function logToPanel(message) {
    //console.log("inside logPanel");
    outputChannel.appendLine(message);
}
// Function to get the program name from the user
// async function getProgramName() {
//   const fileName = await vscode.window.showInputBox({
//     placeHolder:
//       "Please enter the name of a futil file in the workspace folder",
//     value: "default.futil",
//   });
//   if (fileName) {
//     if (!fileName.startsWith("/")) {
//       const path = require("path");
//       return path.join(
//         vscode.workspace.workspaceFolders[0].uri.fsPath,
//         fileName
//       );
//     }
//     return fileName;
//   } else {
//     return null;
//   }
// }
// Factory for multi-session
var CiderDebugAdapterDescriptorFactoryServer = /** @class */ (function () {
    function CiderDebugAdapterDescriptorFactoryServer(adapterPath, stdPath, workspace, outputChannel) {
        logToPanel("inside constructor");
        this.adapter = new CiderDebugAdapter(adapterPath, stdPath, workspace, outputChannel);
        this.stdPath = stdPath;
        this.adapterPath = adapterPath;
        this.workspace = workspace;
        this.outputChannel = outputChannel;
    }
    CiderDebugAdapterDescriptorFactoryServer.prototype.createDebugAdapterDescriptor = function (session) {
        // Return a new debug adapter descriptor
        logToPanel("line 57: create_DA_Desc");
        // default port: 8888
        var port = vscode.workspace.getConfiguration("cider-dap").port;
        // adjust this logic to use promises too
        if (!this.adapter.isServerRunning()) {
            var adapterPromise = this.adapter.start(port);
            return adapterPromise.then(function (res) {
                logToPanel("line 66: connect to debugger");
                return new vscode.DebugAdapterServer(res);
            }, function () { throw "Failed to start debug server"; });
        }
        else {
            logToPanel("line 71: connect to debugger");
            return new vscode.DebugAdapterServer(port);
        }
    };
    return CiderDebugAdapterDescriptorFactoryServer;
}());
var CiderDebugAdapter = /** @class */ (function () {
    function CiderDebugAdapter(adapterPath, stdPath, cwd, outputChannel) {
        logToPanel("line 83: CDA constructor start");
        this.adapterPath = adapterPath;
        this.stdPath = stdPath;
        this.cwd = cwd;
        this.outputChannel = outputChannel;
        this.adapterProcess = null;
        logToPanel("line 89: CDA constructor end");
    }
    CiderDebugAdapter.prototype.isServerRunning = function () {
        logToPanel("line 92: checking if server is running");
        return this.adapterProcess != null && this.adapterProcess.exitCode == null;
    };
    // Start the debug adapter process
    CiderDebugAdapter.prototype.start = function (port) {
        var _this = this;
        logToPanel("line 97: CDA start(port)");
        // Spawn a new child process for the debug adapter
        // Include the port as a command line argument
        return new Promise(function (resolve, reject) {
            _this.adapterProcess = cp.spawn(_this.adapterPath, ["--port", port, "--tcp", "-l", _this.stdPath], { cwd: _this.cwd });
            // Attach event listener to capture standard output of the adapter process and log it to the output channel
            _this.adapterProcess.stdout.on("data", function (data) {
                logToPanel(data.toString());
            });
            // Attach event listener to capture standard error of the adapter process and log it to the output channel
            _this.adapterProcess.stderr.on("data", function (data) {
                logToPanel(data.toString());
            });
            _this.adapterProcess.on("spawn", function () {
                logToPanel("Debugger started on port " + port + "!");
                setTimeout(function () { return resolve(port); }, 200);
            });
            _this.adapterProcess.on("error", function () {
                logToPanel("Debugger failed to start");
                reject(-1);
            });
        });
    };
    CiderDebugAdapter.prototype.stop = function () {
        if (this.adapterProcess) {
            this.adapterProcess.kill();
            this.adapterProcess = null;
            this.isRunning = false;
            logToPanel("Debugger stopped.");
        }
        else {
            logToPanel("No running debug adapter to stop.");
        }
    };
    return CiderDebugAdapter;
}());
// Factory for single-session
var CiderDebugAdapterDescriptorFactoryExecutable = /** @class */ (function () {
    function CiderDebugAdapterDescriptorFactoryExecutable() {
    }
    CiderDebugAdapterDescriptorFactoryExecutable.prototype.createDebugAdapterDescriptor = function (_session) {
        // Use the DebugAdapterExecutable as the debug adapter descriptor
        console.log("inside adapter factory");
        console.log(vscode.workspace.getConfiguration("cider-dap").path);
        return new vscode.DebugAdapterExecutable(vscode.workspace.getConfiguration("cider-dap").path, [], { cwd: vscode.workspace.rootPath });
    };
    return CiderDebugAdapterDescriptorFactoryExecutable;
}());
function activate(context) {
    logToPanel("Extension activated!");
    var factory;
    // Get session type (multi or single) from package.json configuration
    logToPanel("setting up with configuration '" + vscode.workspace.getConfiguration("cider-dap").sessionType + "'. You will need to reload after changing the settings if a different mode is desired.");
    switch (vscode.workspace.getConfiguration("cider-dap").sessionType) {
        case "Multi-Session":
            factory = new CiderDebugAdapterDescriptorFactoryServer(vscode.workspace.getConfiguration("cider-dap").path, vscode.workspace.getConfiguration("cider-dap").std_lib, vscode.workspace.rootPath, outputChannel);
            break;
        case "Single-Session":
        default:
            factory = new CiderDebugAdapterDescriptorFactoryExecutable();
            break;
    }
    context.subscriptions.push(vscode.debug.registerDebugAdapterDescriptorFactory("cider-dap", factory));
    logToPanel("after start server");
    // Update the adapter path with the serverPort and use it for starting the debug adapter
    logToPanel("before startDebugging");
    logToPanel("Hello, your extension is now activated!");
}
function stopDebugging() {
    if (debugAdapter) {
        debugAdapter.stop();
    }
    else {
        logToPanel("No running debug adapter to stop.");
    }
}
function deactivate() {
    //kill server
    logToPanel("deactivate");
}
module.exports = {
    activate: activate,
    deactivate: deactivate,
};
