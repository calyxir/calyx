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

- [fud: The Calyx Driver](./running-calyx/fud/index.md)
  - [Examples](./running-calyx/fud/examples.md)
  - [Xilinx Tools](./running-calyx/fud/xilinx.md)
    - [AXI Generation](./running-calyx/fud/axi-gen.md)
  - [External Stages](./running-calyx/fud/external.md)
  - [Multiple Paths](./running-calyx/fud/multiple-paths.md)
  - [CIRCT](./running-calyx/fud/circt.md)
  - [Resource Estimation](./running-calyx/fud/resource-estimation.md)
- [fud2: Experimental Driver](./running-calyx/fud2.md)
- [Interfacing with Calyx RTL](./running-calyx/interfacing.md)
- [The Calyx Interpreter](./running-calyx/interpreter.md)

# Compiler Development Guide

- [The Calyx Compiler](./compiler.md)
- [Adding a New Pass](./new-pass.md)
- [Primitive Library](./libraries/core.md)
- [The `calyx` Library](./compiler-as-library.md)
- [Dataflow Analysis](./optimizations/dataflow.md)
- [Debugging](./debug/index.md)
  - [Logical Bugs](./debug/cider.md)
  - [Compilation Bugs](./debug/debug.md)
- [Contributing to Calyx](./github.md)

# Generating Calyx

- [Emitting Calyx from Python](./builder/calyx-py.md)
  - [Builder Library Walkthrough](./builder/walkthrough.md)
- [Frontend Tutorial](./tutorial/frontend-tut.md)
- [Frontend Compilers](./frontends/index.md)
  - [Dahlia](./frontends/dahlia.md)
  - [Systolic Array Generator](./frontends/systolic-array.md)
  - [TVM Relay](./frontends/tvm-relay.md)
  - [NTT Pipeline Generator](./frontends/ntt.md)
  - [Queues](./frontends/queues.md)
  - [MrXL](./frontends/mrxl.md)

# Tools

- [Runt](./tools/runt.md)
- [Data Gen](./tools/data-gen.md)
- [`exp` Generator](./tools/exp-generator.md)
- [Editor Highlighting](./tools/editor-highlighting.md)
- [Language Server](./tools/language-server.md)

----
[Contributors](./contributors.md)
