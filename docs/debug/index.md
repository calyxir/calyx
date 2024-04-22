# Debugging

Calyx programs can provide wrong answers for two reasons:
1. The program implements the wrong algorithm, i.e., it has a logical bug.
2. The Calyx compiler incorrectly compiles the program, i.e., there is a compilation bug.

First make sure that the program generates the correct values with the [Calyx
Interpreter](../interpreter.md). If it produces the wrong values, your Calyx implementation of the
algorithm is incorrect. You can use the [Calyx Debugger](./cider.md) to debug these problems.

If the interpreter produces the right values, try a different Verilog backed. We support both
[Verilator](../running-calyx/fud/index.md#verilator) and [Icarus Verilog](../running-calyx/fud/index.md#icarus-verilog). If
both produce the wrong answer *and* the interpreter produces the right answer then you likely have
a compilation bug on your hands. Use the [debugging tips][tips] to narrow down the pass that causes
the error.

[tips]: ./debug.md
[cidr]: ./cider.md
