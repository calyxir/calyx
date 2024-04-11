# Cider Debug Adapter

## Installing the extension

Navigate to your vscode extension directory `~/.vscode/extensions` for local
installations. For WSL users, you need to use the _server's_ extension folder not
the normal installation in windows. Once in the appropriate folder create a
symlink for the extension

```
ln -s  <PATH TO CALYX ROOT>/cider-dap/calyxDebug cider.cider-dap-0.0.1
```

Once vscode is reloaded, the extension should be active and viewable in the
`Cider dap` tab of the output panel. You will also need to create a symlink for
the cider-dap binary somewhere on your path. From some directory on your PATH:

```
ln -s <PATH TO CALYX ROOT>/target/debug/cider-dap
```

You will have to configure user settings of cider-dap in VSCode and input your cider binary path, calyx std_lib path, session type, and port number (if debug adapter is started as a server). You can then launch the adapter with the Debug w/ Cider action.

## Known issues

- The launch action can sometimes attempt a connection before the server is
  ready and will cause a failure, subsequent attempts will work until the
  server closes. Ideally the extension would wait until the server has fully launched.

## Project Overview:

Cider-dap is a debug adapter protocol implementation using Rust, which aids in debugging processes. It interfaces with IDEs or other tools using the Debug Adapter Protocol.
This project primarily leverages the Debug Adapter Protocol (DAP) for its functionality. The structure is organized into different directories and files which encapsulate the functionalities:
<br>
<br> 1.`cider-dap` directory: The main directory which contains the following sub-directories and files:
<br>
&nbsp; &nbsp; &nbsp; &nbsp; `calyxDebug`: Contains the file responsible for debugging extensions and related utilities. So it is a dedicated directory for VSCode debugging extensions. It establishes the bridge between your Rust codebase and the VSCode debugging environment. <br>
&nbsp; &nbsp; &nbsp; &nbsp; `src`: Houses the Rust source files for the project. It contains the project's core functionalities, logic, and structures. <br>
&nbsp; &nbsp; &nbsp; &nbsp; `cargo.lock` & `cargo.toml`: Standard Rust project files detailing dependencies and project metadata. <br> 3. `src` directory: <br>
&nbsp; &nbsp; &nbsp; &nbsp; `adapter.rs`: Defines the primary adapter structure for the project and its associated functionalities. Not just any adapter, this file structures the fundamental protocols, handling the translation of high-level debugging commands into actionable, low-level instructions. <br>
&nbsp; &nbsp; &nbsp; &nbsp; `error.rs`: Contains custom error definitions and types for better error handling. <br>
&nbsp; &nbsp; &nbsp; &nbsp; `main.rs`: The entry point for the project, it integrates functionalities from the other source files and provides the main execution logic. <br> 4. `calyxDebug` directory: <br>
&nbsp; &nbsp; &nbsp; &nbsp; `extension.ts`: TypeScript file for VSCode extension integration. It provides functions to interface between the VSCode environment and the Rust backend. <br>

### About main.rs: the Main File

In `main.rs`, our program is set up to accommodate both single and multi-session debugging. It defines the entry point of our application and orchestrates how the server is run based on user-provided arguments.

#### Initialization:

At the start of the `main()` function:

- Initializes a logger to log to the terminal if in multi-session and a file in single-session.
- The Opts struct captures command-line arguments. This struct contains an optional file path, a switch to determine if the application runs in multi-session mode, and a port number (with a default of 8080).
- `argh::from_env()` processes the command-line arguments based on the defined struct. The use of argh simplifies command-line parsing, allowing you to focus on the main logic without getting bogged down in argument processing.

#### Single vs. Multi-Session Decision:

Depending on whether the `is_multi_session flag` is set:

##### Multi-Session:

- &nbsp; &nbsp; &nbsp; &nbsp; A TCP listener is set up, which will listen for incoming debugger connections.
- &nbsp; &nbsp; &nbsp; &nbsp; On accepting a connection, the streams are buffered for efficient I/O operations.
- &nbsp; &nbsp; &nbsp; &nbsp; The multi_session_init function gets the adapter configured for the session, handling initial handshakes like the Initialize and Launch commands.
- &nbsp; &nbsp; &nbsp; &nbsp; The run_server function then takes over, orchestrating the actual debugging session with the given adapter.

##### Single-Session:

- &nbsp; &nbsp; &nbsp; &nbsp; Directly reads from standard input and writes to standard output.
- &nbsp; &nbsp; &nbsp; &nbsp; Instead of expecting and processing initial handshakes, the function simply sets up the adapter with the provided file and begins the server loop.

This dual mode is valuable: the single-session approach allows for streamlined debugging in local environments or for simpler setups, while the multi-session setup allows for more advanced scenarios, perhaps remote debugging or handling multiple debugger sessions.

#### `multi_session_init`

This function sets up an adapter for a multi-session environment. Here's a step-by-step breakdown:

##### 1. Initial Handshake:

- It first waits for and processes an Initialize request. This is fundamental in the DAP as it establishes the initial connection between the debugger client and server.
- After successfully processing this request, it sends an Initialized event to notify the client that the server is ready for subsequent commands.

##### 2. Setup:

- The next expected command is a Launch command. This command contains additional information (like the path to the program being debugged). This path is extracted and checked for validity.
- The program is then opened and used to construct the MyAdapter instance.
  The purpose of this function is to perform the initial setup necessary to start a debugging session. By separating it from the run_server function, the code remains modular, allowing for easier debugging, testing, and modification.

#### <font size="3"> run_server </font> :

The heart of the debugger's runtime:

##### <font size="3"> Core Loop </font>:

The function continuously polls for requests from the client.

- Upon receiving a Launch command, it sends a successful response back to the client. This indicates that the server is ready to begin debugging.
- The loop can be expanded to handle other DAP commands as needed. For example, handling a Disconnect command could cleanly terminate the loop and close the server.

##### Command Processing:

- The only command being actively handled right now is the Launch command. Upon receiving this command, the server simply responds with a success message, indicating that it's ready to commence debugging.
- The loop is designed with extensibility in mind. Comments suggest places where commands like Disconnect can be incorporated to handle disconnection events, allowing the server to terminate gracefully.

##### <font size="3"> Error Handling </font>:

- If an unknown command is received, it's printed to the error output and the server terminates with an UnhandledCommandError.
- This is a robust approach, ensuring that only expected commands are processed and any anomalies are immediately flagged.

### Dependencies

The following dependencies have been added to the project as specified in the cargo.toml:
<br>

- `dap`: Rust DAP implementation. At its core, this Rust DAP implementation is what powers cider-dap. It's the backbone that ensures all debugging actions are in line with the protocol's standards. <br>
- `thiserror`: Used for ergonomic error handling. Enhancing error handling by providing more contextual feedback and streamlined debugging. <br>
- `serde_json` & `serde`: Serialization and deserialization of data. Essential for data communication. They ensure that data structures are efficiently serialized and deserialized between different parts of the system. <br>
- `owo-colors`: For colored console output. So it elevates user experience by introducing color-coded outputs, making console interactions more intuitive. <br>
- `argh`: For command line argument parsing. It simplifies command line interactions, ensuring that user inputs are effectively parsed and processed. <br>

### Running the Project

1. Ensure you have the necessary dependencies installed. If not, you can install them using cargo:
   `cargo install `
2. To run the main project:
   `cargo run `

### Next Steps

1. Advanced Error Handling: Utilize the structures in error.rs to provide detailed insights, potentially integrating with external error databases or logs.
2. Command Enhancements: Augment the DAP commands and responses in main.rs, exploring beyond traditional debugging actions.
3. There are changes needed to be done inside run_server:

### Additional Command Handling:

- Incorporate command handlers for other DAP commands:
- `Disconnect `: Handle disconnect commands gracefully, ensuring any necessary cleanup is done before closing the server.
- `Breakpoint `: Implement functionality to pause execution at specific points.
- `StepOver `, `StepInto `, `StepOut `: Allow fine-grained control over the debugging process, allowing users to inspect code step-by-step.
- `Evaluate `: Handle evaluation requests from the debugger, returning values as needed.

### Refined Error Handling:

- Instead of immediate termination on an unknown command, consider logging these events or sending specific error messages back to the client. This provides a more user-friendly debugging experience.

### Enhanced Logging:

- Implement more detailed logging to provide insights into server operations. This would be especially useful in identifying issues or understanding the flow of commands and responses.

### Asynchronous Processing:

- Consider incorporating asynchronous command processing. This would allow the server to handle multiple requests concurrently, offering faster response times and smoother user experiences, especially in complex debugging scenarios.
