[Getting Started](./intro.md)

# Calyx Language

- [Language Tutorial](./tutorial/language-tut.md)
  - [Multi-Component Designs](./lang/multi-component.md)
  - [Passing Memories by Reference](./lang/memories-by-reference.md)
- [Language Reference](./lang/ref.md)
  - [Data Format](./lang/data-format.md)
  - [Static Timing](./lang/static.md)
  - [Experimental: Synchronization](./lang/sync.md)
  - [Undefined Behaviors](./lang/undefined.md)
- [Attributes](./lang/attributes.md)

# Running Calyx Programs

- [`fud`: The Calyx Driver](./fud/index.md)
  - [Examples](./fud/examples.md)
  - [Xilinx Tools](./fud/xilinx.md)
    - [AXI Generation](./fud/axi-gen.md)
  - [External Stages](./fud/external.md)
  - [Multiple Paths](./fud/multiple-paths.md)
  - [CIRCT](./fud/circt.md)
  - [Resource Estimation](./fud/resource-estimation.md)
- [The Calyx Interpreter](./interpreter.md)

# Compile Development Guide

- [The Calyx Compiler](./compiler.md)
- [Adding a New Pass](./new-pass.md)
- [Primitive Library](./libraries/core.md)
- [The `calyx` Library](./compiler-as-library.md)
- [Dataflow Analysis](./optimizations/dataflow.md)
- [Debugging](./debug/index.md)
  - [Logical Bugs](./debug/cider.md)
  - [Compilation Bugs](./debug/debug.md)

# Generating Calyx

- [Emitting Calyx from Python](./builder/calyx-py.md)
  - [`calyx-py` Builder Reference](./builder/ref.md)
- [Frontend Tutorial](./tutorial/frontend-tut.md)
- [Frontend Compilers](./frontends/index.md)
  - [Dahlia](./frontends/dahlia.md)
  - [Systolic Array Generator](./frontends/systolic-array.md)
  - [TVM Relay](./frontends/tvm-relay.md)
  - [NTT Pipeline Generator](./frontends/ntt.md)
  - [MrXL](./frontends/mrxl.md)

# Tools

- [Runt](./tools/runt.md)
- [Data Gen](./tools/data-gen.md)
- [`exp` Generator](./tools/exp-generator.md)
- [Editor Highlighting](./tools/editor-highlighting.md)

----
[Contributors](./contributors.md)
