## About the Name?
cider-dap is a sub-crate created for facilitating debugging processes. The name is inspired by the core name "Cider", and "dap" stands for Debug Adapter Protocol.
### Project Overview:
Cider-dap is a debug adapter protocol implementation using Rust, which aids in debugging processes. It interfaces with IDEs or other tools using the Debug Adapter Protocol.
This project primarily leverages the Debug Adapter Protocol (DAP) for its functionality. The structure is organized into different directories and files which encapsulate the functionalities:
1. cider-dap directory: The main directory which contains the following sub-directories and files:
     calyxDebug: Contains the file responsible for debugging extensions and related utilities.
     src: Houses the Rust source files for the project.
     cargo.lock & cargo.toml: Standard Rust project files detailing dependencies and project metadata.
2. src directory:
     adapter.rs: Defines the primary adapter structure for the project and its associated functionalities.
     error.rs: Contains custom error definitions and types for better error handling.
     main.rs: The entry point for the project, it integrates functionalities from the other source files and provides the main execution logic.
3. calyxDebug directory:
     extension.js: JavaScript file for VSCode extension integration. It provides functions to interface between the VSCode environment and the Rust backend.

### Dependencies
The following dependencies have been added to the project as specified in the cargo.toml:

- dap: Rust DAP implementation.
- thiserror: Used for ergonomic error handling.
- serde_json & serde: Serialization and deserialization of data.
- owo-colors: For colored console output.
- argh: Command line argument parsing.

### Running the Project
1. Ensure you have the necessary dependencies installed. If not, you can install them using cargo:
2. To run the main project:

### Next Steps

- Enhance error handling by utilizing the custom errors defined in error.rs.
- Implement more commands and responses in main.rs to fully utilize the capabilities of DAP.
- Improve the VSCode extension in calyxDebug for a richer debugging experience.
