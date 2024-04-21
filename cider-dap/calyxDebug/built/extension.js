"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __generator = (this && this.__generator) || function (thisArg, body) {
    var _ = { label: 0, sent: function() { if (t[0] & 1) throw t[1]; return t[1]; }, trys: [], ops: [] }, f, y, t, g;
    return g = { next: verb(0), "throw": verb(1), "return": verb(2) }, typeof Symbol === "function" && (g[Symbol.iterator] = function() { return this; }), g;
    function verb(n) { return function (v) { return step([n, v]); }; }
    function step(op) {
        if (f) throw new TypeError("Generator is already executing.");
        while (_) try {
            if (f = 1, y && (t = op[0] & 2 ? y["return"] : op[0] ? y["throw"] || ((t = y["return"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;
            if (y = 0, t) op = [op[0] & 2, t.value];
            switch (op[0]) {
                case 0: case 1: t = op; break;
                case 4: _.label++; return { value: op[1], done: false };
                case 5: _.label++; y = op[1]; op = [0]; continue;
                case 7: op = _.ops.pop(); _.trys.pop(); continue;
                default:
                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) { _ = 0; continue; }
                    if (op[0] === 3 && (!t || (op[1] > t[0] && op[1] < t[3]))) { _.label = op[1]; break; }
                    if (op[0] === 6 && _.label < t[1]) { _.label = t[1]; t = op; break; }
                    if (t && _.label < t[2]) { _.label = t[2]; _.ops.push(op); break; }
                    if (t[2]) _.ops.pop();
                    _.trys.pop(); continue;
            }
            op = body.call(thisArg, _);
        } catch (e) { op = [6, e]; y = 0; } finally { f = t = 0; }
        if (op[0] & 5) throw op[1]; return { value: op[0] ? op[1] : void 0, done: true };
    }
};
Object.defineProperty(exports, "__esModule", { value: true });
var vscode = require("vscode");
var cp = require("child_process");
// Hold the debug adapter instance
var debugAdapter = null;
var programName = null; // Store the program name
// Create output channel
var outputChannel = vscode.window.createOutputChannel("Cider dap");
function logToPanel(message) {
    console.log("inside logPanel");
    outputChannel.appendLine(message);
}
// Function to get the program name from the user
function getProgramName() {
    return __awaiter(this, void 0, void 0, function () {
        var fileName, path;
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0: return [4 /*yield*/, vscode.window.showInputBox({
                        placeHolder: "Please enter the name of a futil file in the workspace folder",
                        value: "default.futil",
                    })];
                case 1:
                    fileName = _a.sent();
                    if (fileName) {
                        if (!fileName.startsWith("/")) {
                            path = require("path");
                            return [2 /*return*/, path.join(vscode.workspace.workspaceFolders[0].uri.fsPath, fileName)];
                        }
                        return [2 /*return*/, fileName];
                    }
                    else {
                        return [2 /*return*/, null];
                    }
                    return [2 /*return*/];
            }
        });
    });
}
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
        logToPanel("creating adapter descriptor");
        return new vscode.DebugAdapterServer(this._startDebugServer(session));
    };
    CiderDebugAdapterDescriptorFactoryServer.prototype._startDebugServer = function (session) {
        logToPanel("start of startDebugServer");
        // default port: 8888
        var port = vscode.workspace.getConfiguration("cider-dap").port;
        if (!this.adapter.isServerRunning()) {
            logToPanel("server is not running");
            this.adapter.start(port);
            logToPanel("started dap-server");
        }
        logToPanel("exiting startDebugging");
        return port;
    };
    return CiderDebugAdapterDescriptorFactoryServer;
}());
var CiderDebugAdapter = /** @class */ (function () {
    function CiderDebugAdapter(adapterPath, stdPath, cwd, outputChannel) {
        logToPanel("inside CiderDebugAdapter");
        this.adapterPath = adapterPath;
        this.stdPath = stdPath;
        this.cwd = cwd;
        this.outputChannel = outputChannel;
        this.adapterProcess = null;
        logToPanel("at the end of ciderDebugAdapter");
    }
    CiderDebugAdapter.prototype.isServerRunning = function () {
        logToPanel("checking if server is running");
        return this.adapterProcess != null && this.adapterProcess.exitCode == null;
    };
    // Start the debug adapter process
    CiderDebugAdapter.prototype.start = function (port) {
        logToPanel("beginning of start");
        // Spawn a new child process for the debug adapter
        // Include the port as a command line argument
        this.adapterProcess = cp.spawn(this.adapterPath, ["--port", port, "--tcp", "-l", this.stdPath], { cwd: this.cwd });
        // Attach event listener to capture standard output of the adapter process and log it to the output channel
        this.adapterProcess.stdout.on("data", function (data) {
            logToPanel(data.toString());
        });
        // Attach event listener to capture standard error of the adapter process and log it to the output channel
        this.adapterProcess.stderr.on("data", function (data) {
            logToPanel(data.toString());
        });
        this.adapterProcess.on("spawn", function () {
            logToPanel("Debugger started on port " + port + "!");
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
    logToPanel("deactivate");
}
module.exports = {
    activate: activate,
    deactivate: deactivate,
};
