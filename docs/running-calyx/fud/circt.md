# CIRCT

An ongoing effort is under way to establish Calyx as a dialect in the LLVM umbrella project [CIRCT][].
There is documentation about the Calyx dialect [on the MLIR site][calyx-dialect]. While semantically
equivalent, they are syntactically different.  Because the Calyx dialect is still under progress and
does not include all the optimizations that the native Rust compiler supports, we have crafted an emitter
from the Calyx dialect (MLIR) to the native compiler representation (used by the Rust compiler). This means
you can lower from your favorite frontend in MLIR to the Calyx dialect, and continue all the way to
SystemVerilog (with spunky optimizations) using the native compiler.

The native compiler also supports round-tripping back into the MLIR representation. We'll assume you've
already built the Rust compiler and installed `fud`. Here are the steps below to round-trip:

## MLIR to Native Representation
1. Set up the CIRCT project with [these instructions][circt-setup].

2. There should be a `circt-translate` binary in `<root-directory>/build/bin`. To emit the native compiler
   representation, use the command:
```
path/to/circt-translate --export-calyx /path/to/file
```

For example, you can use the expected output of the test `tests/backend/mlir/simple.expect`:
```
{{#include ../../tests/backend/mlir/simple.expect}}
```

Using the command:

```bash
# Don't worry too much about the file alias; this is used for testing purposes.
path/to/circt-translate --export-calyx tests/backend/mlir/simple.expect
```

This should output:

```
{{#include ../../tests/backend/mlir/simple.futil}}
```

## Native Representation to MLIR
To round-trip back to the Calyx dialect, we can use `fud`:
```sh
fud exec path/to/file --to mlir
```

For example,
```sh
fud exec tests/backend/mlir/simple.futil --to mlir
```

This should emit the Calyx dialect once again.

## Using Native Tools with MLIR-Generated Calyx

The native infrastructure, such as [`fud`], [the calyx debugger][cider], our [synthesis scripts][synth], and the [AXI generator][axi-gen] all make certain assumptions that are violated by MLIR-generated code.
Specifically, the tools often require that:
1. The interface memories are marked with the [`@external`][] attribute. This allows our testbench to generate the code needed by [`fud`][] to simulate designs with the convenient data format. It is also used by the [AXI generator][axi-gen] to generate AXI interfaces for memories.
2. The `toplevel` component is named `main`. This is used by the [synthesis scripts][synth] to generate resource usage numbers and the test bench to simulate the design.

While we're working on addressing these problems directly, in the meantime, if you'd like to use the native tools with MLIR-generated code, you can use the following two passes:
1. `discover-external` which transforms MLIR's representation of interface memories into `@external` memories.
2. `wrap-main` which adds a `main` component to the program and makes it the entrypoint component. This pass is enabled by default.

An example invocation of these passes is:
```sh
calyx <file> -p validate -p discover-external -p all -x discover-external:default=4
```

The `-p discover-external` flag enables the pass to transform ports into interface memories. Unfortunately, this process is not fully automatic.
For example, it is not possible for the pass to infer the size of your memories by just looking at the signals provided to the top-level component.
We provide `-x discover-external:default=<size>` which tells the pass that when you cannot infer the size parameter of a memory, use `<size>` as the default.
A limitation of this approach is that the pass does not support discovering interface memories with different sizes.
If you desperately need this, please [open an issue][issue], and we'll try to prioritize it.

[`fud`]: ./index.md
[cider]: ../interpreter.md
[synth]: ./xilinx.html#synthesis-only
[axi-gen]: ./axi-gen.html
[`@external`]: ../lang/attributes.html#external
[issue]:https://github.com/calyxir/calyx/issues/new

[circt]: https://circt.llvm.org/
[circt-setup]: https://github.com/llvm/circt#setting-this-up
[calyx-dialect]: https://circt.llvm.org/docs/Dialects/Calyx/
