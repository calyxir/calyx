# Calyx Interpreter

This is the interpreter for Calyx, implemented in Rust. Currently, it is rather limited; it can only interpret a single group and has very limited functionality.

## Usage:
`cargo run -- -c <component name> -g <group name> <input file>`

where `component name` is an optional argument for the name of the component (default is `main`),

`group name` is a required argument for the name of the group,

and `input file` is a Calyx program (note: the interpreter currently cannot handle multi-component programs).

Examples:
`cargo run -- -c main -g op ./tests/simple_add.futil`
