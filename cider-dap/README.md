## About the Name?
Inspired by the comforting essence of apple cider, our sub-crate tries to bring some of that warmth to debugging. Now, onto what this project is and what's been brewing!

cider-dap is a sub-crate created for facilitating debugging processes. The name is inspired by the core name "Cider", and "dap" stands for Debug Adapter Protocol!
### Project Overview:
Cider-dap is a debug adapter protocol implementation using Rust, which aids in debugging processes. It interfaces with IDEs or other tools using the Debug Adapter Protocol.
This project primarily leverages the Debug Adapter Protocol (DAP) for its functionality. The structure is organized into different directories and files which encapsulate the functionalities: 
<br>
<br>
1.``` cider-dap ``` directory: The main directory which contains the following sub-directories and files:
<br> 
     ```calyxDebug```: Contains the file responsible for debugging extensions and related utilities. So it is a dedicated directory for VSCode debugging extensions. It establishes the bridge between your Rust codebase and the VSCode debugging environment. <br> 
     ```src```: Houses the Rust source files for the project. It contains the project's core functionalities, logic, and structures. <br> 
     ```cargo.lock``` & ```cargo.toml```: Standard Rust project files detailing dependencies and project metadata. <br>
3. ```src``` directory: <br>
     ```adapter.rs```: Defines the primary adapter structure for the project and its associated functionalities. Not just any adapter, this file structures the fundamental protocols, handling the translation of high-level debugging commands into actionable, low-level instructions. <br>
     ```error.rs```: Contains custom error definitions and types for better error handling. <br>
     ```main.rs```: The entry point for the project, it integrates functionalities from the other source files and provides the main execution logic. <br>
4. ```calyxDebug``` directory: <br>
     ```extension.js```: JavaScript file for VSCode extension integration. It provides functions to interface between the VSCode environment and the Rust backend. <br>

### Dependencies
The following dependencies have been added to the project as specified in the cargo.toml:
<br>
- ```dap```: Rust DAP implementation.  At its core, this Rust DAP implementation is what powers cider-dap. It's the backbone that ensures all debugging actions are in line with the protocol's standards. <br>
- ```thiserror```: Used for ergonomic error handling. Enhancing error handling by providing more contextual feedback and streamlined debugging. <br>
- ```serde_json``` & ```serde```: Serialization and deserialization of data. Essential for data communication. They ensure that data structures are efficiently serialized and deserialized between different parts of the system. <br>
- ```owo-colors```: For colored console output. So it elevates user experience by introducing color-coded outputs, making console interactions more intuitive. <br>
- ```argh```: For command line argument parsing. It simplifies command line interactions, ensuring that user inputs are effectively parsed and processed. <br>

### Running the Project
1. Ensure you have the necessary dependencies installed. If not, you can install them using cargo:
 ```cargo install ```
3. To run the main project:
```cargo run ```

### Next Steps

1. Advanced Error Handling: Utilize the structures in error.rs to provide detailed insights, potentially integrating with external error databases or logs.
2. Command Enhancements: Augment the DAP commands and responses in main.rs, exploring beyond traditional debugging actions.

