# Calyx Compiler as a Library

The Calyx compiler is separated into [multiple crates][crates] that can be used independently.
If you're interested in adding a new pass to the Calyx compiler or build a tool using it, your best bet is to [take a look at the example in the `calyx-opt`][opt-ex] library.

The `calyx` implements the compiler driver and plumbs together all the other crates.
You mostly likely want to include the `calyx-opt` crate if you're working passes or just the `calyx-ir` crate if you're working with the IR.
You'll also need `calyx-frontend` and `calyx-utils` if you're parsing frontend code.


[crates]: https://docs.rs/releases/search?query=calyx
[opt-ex]: https://docs.rs/calyx-opt/0.2.1/calyx_opt/