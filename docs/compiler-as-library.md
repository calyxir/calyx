# Calyx Compiler as a Library

The Calyx compiler is separated into [multiple crates][crates] that can be used independently.
If you're interested in adding a new pass to the Calyx compiler or build a tool using it, your best bet is to [take a look at the example in the `calyx-opt`][opt-ex] library.

The `calyx` implements the compiler driver and plumbs together all the other crates.
You mostly likely want to include the `calyx-opt` crate if you're working passes or just the `calyx-ir` crate if you're working with the IR.
You'll also need `calyx-frontend` and `calyx-utils` if you're parsing frontend code.

## Building the `calyx` Binary

The [`calyx` binary][calyx-crate] is published using Rust's crates.io repository. It provides the [compiler interface](./compiler.md) which can be used without requiring the user to build the compiler from source. The `calyx` binary also ships all its [primitives library][prims-lib] which is done through a somewhat complex bootstrapping process (see [#1678](https://github.com/calyxir/calyx/pull/1678))

1. The [`calyx-stdlib`][calyx-stdlib] package pulls in the sources of all the primitives using the Rust `include_str!` macro.
2. The `calyx` binary defines a build script that depends on `calyx-stdlib` as a build dependency.
3. During build time, the script loads the string representation of all the primitives files and writes them to `$CALYX_PRIMITIVE_DIR/primitives`. If the variable is not set, the location defaults to `$HOME/.calyx`.
4. If (3) succeeds, the build scripts defines the `CALYX_PRIMITIVES_LIB` environment variable which is used when compiling the `calyx` crate.
5. During compilation, `calyx` embeds the value of this environment variable as the default argument to the `-l` flag. If the variable is not defined, the default value of the `-l` flag is `.`.

Users of the `calyx` binary can still specify a value for `-l` to override the default primitives file. For example, the `fud` configuration for the `calyx` stage override the value of `-l` to the location of the Calyx repo.


[crates]: https://docs.rs/releases/search?query=calyx
[opt-ex]: https://docs.rs/calyx-opt/0.2.1/calyx_opt/
[calyx-crate]: https://crates.io/crates/calyx
[prims-lib]: ./libraries/core.md
[calyx-stdlib]: https://crates.io/crates/calyx-stdlib