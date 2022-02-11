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

[circt]: https://circt.llvm.org/
[circt-setup]: https://github.com/llvm/circt#setting-this-up
[calyx-dialect]: https://circt.llvm.org/docs/Dialects/Calyx/
