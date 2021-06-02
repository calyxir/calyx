# Compiler Development

The Calyx compiler is organized as a set of passes. At a high-level the
compiler:

- Parses the program.
- Transforms it into an internal representation.
- Checks if it is well-formed.
- Applies optimization passes.
- Removes all control from the program.
- Emits SystemVerilog.
