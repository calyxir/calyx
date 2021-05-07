# FuTIL Interpreter

This is the interpreter for FuTIL, implemented in Rust. Currently, it is rather limited; it can only interpret a single group and has very limited functionality. For example, it cannot interpret multi-component programs.

## Usage:
`cargo run -- -c <component name> -g <group name> <input file>`

where `component name` is an optional argument for the name of the component (default is `main`),

`group name` is an optional argument for the name of the group (default is `main`; currently does not affect output),

and `input file` is a FuTIL program.

Examples:
`cargo run -- ./tests/simple_add.futil`
