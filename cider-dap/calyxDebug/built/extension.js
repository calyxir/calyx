"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var vscode = require("vscode");
var cp = require("child_process");
// Hold the debug adapter instance
var debugAdapter = null;
// Create output channel
var outputChannel = vscode.window.createOutputChannel("Cider dap");
var r = new String(vscode.workspace.workspaceFolders[0].uri);
var root = r.substring(r.indexOf("://") + 3).toString();
var disposables = [];
function logToPanel(message) {
    outputChannel.appendLine(message);
}
var CiderDebugAdapterDescriptorFactoryServer = /** @class */ (function () {
    function CiderDebugAdapterDescriptorFactoryServer() {
    }
    CiderDebugAdapterDescriptorFactoryServer.prototype.createDebugAdapterDescriptor = function (session, executable) {
        var stdPath = vscode.workspace.getConfiguration("cider-dap").std_lib;
        var adapterPath = vscode.workspace.getConfiguration("cider-dap").path; //tried just using the executable.command but that doesnt work idk why
        var port = vscode.workspace.getConfiguration("cider-dap").port;
        this.adapter = new CiderDebugAdapter(adapterPath, stdPath, root, outputChannel);
        if (!this.adapter.isServerRunning()) {
            logToPanel("adapter descripter: calling adapter.start(port)");
            var adapterPromise = this.adapter.start(port);
            return adapterPromise.then(function (res) {
                return new vscode.DebugAdapterServer(res);
            }, function () { throw "Failed to start debug server"; });
        }
        else {
            return new vscode.DebugAdapterServer(port);
        }
    };
    CiderDebugAdapterDescriptorFactoryServer.prototype.dispose = function () {
        logToPanel("disposed multi session factory");
        if (this.adapter) {
            this.adapter.stop();
        }
    };
    return CiderDebugAdapterDescriptorFactoryServer;
}());
// Factory for single-session
var CiderDebugAdapterDescriptorFactoryExecutable = /** @class */ (function () {
    function CiderDebugAdapterDescriptorFactoryExecutable() {
    }
    CiderDebugAdapterDescriptorFactoryExecutable.prototype.createDebugAdapterDescriptor = function (session, executable) {
        // Use the DebugAdapterExecutable as the debug adapter descriptor
        return new vscode.DebugAdapterExecutable(vscode.workspace.getConfiguration("cider-dap").path, [], { cwd: root });
    };
    CiderDebugAdapterDescriptorFactoryExecutable.prototype.dispose = function () {
        logToPanel("disposed single session factory");
    };
    return CiderDebugAdapterDescriptorFactoryExecutable;
}());
var CiderDebugAdapter = /** @class */ (function () {
    function CiderDebugAdapter(adapterPath, stdPath, cwd, outputChannel) {
        this.adapterPath = adapterPath;
        this.stdPath = stdPath;
        this.cwd = cwd;
        this.outputChannel = outputChannel;
        this.adapterProcess = null;
    }
    CiderDebugAdapter.prototype.isServerRunning = function () {
        return this.adapterProcess != null && this.adapterProcess.exitCode == null;
    };
    // Start the debug adapter process
    CiderDebugAdapter.prototype.start = function (port) {
        var _this = this;
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
                setTimeout(function () { return resolve(port); }, 200); //short wait to let the thing start running 
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
function activate(context) {
    logToPanel("Extension activated!");
    var factory;
    // Get session type (multi or single) from package.json configuration
    logToPanel("setting up with configuration '" + vscode.workspace.getConfiguration("cider-dap").sessionType + "'. You will need to reload after changing the settings if a different mode is desired.");
    switch (vscode.workspace.getConfiguration("cider-dap").sessionType) {
        case "Multi-Session":
            factory = new CiderDebugAdapterDescriptorFactoryServer();
            break;
        case "Single-Session":
        default:
            factory = new CiderDebugAdapterDescriptorFactoryExecutable();
            break;
    }
    context.subscriptions.push(vscode.debug.registerDebugAdapterDescriptorFactory("cider-dap", factory));
    disposables.push(vscode.debug.registerDebugAdapterDescriptorFactory("cider-dap", factory));
    // Update the adapter path with the serverPort and use it for starting the debug adapter - ??
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
    // when is this called i don't see it on the output logs
    logToPanel("deactivate");
    disposables.forEach(function (d) { return d.dispose(); });
}
module.exports = {
    activate: activate,
    deactivate: deactivate,
};
