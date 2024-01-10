# Interfacing with Calyx Programs

In order to run RTL designs created from Calyx programs, toplevel `reset` and `go`
signals must be interfaced with correctly.

Namely:
1. The `reset` signal must be asserted then deasserted, to initialize the state inside
control registers correctly.
2. The `go` signal must be asserted for as long as the module is running. Deasserting
the `go` signal before a component's `done` signal is asserted will lead to
[undefined behavior][go-done].

Asserting the `reset` and `go` signals in this order is important. Otherwise the toplevel
component will begin running with garbage data inside of control registers.


Interfacing with RTL designs in this way becomes relevant when writing harnesses/testbenches
to execute programs created with Calyx.

## Cocotb

As a concrete example, consider using [cocotb][]
to test a Calyx-generated Verilog design.

If we imagine a simple Calyx program that contains a simple toplevel module named `main`:

```
component main()->() {
    cells {
        reg = std_reg(4);
    }
    group write_to_reg {
        reg.in = 4'd3;
        reg.write_en = 1'b1;
        write_to_reg[done] = reg.done;
    }
    control{
        seq{
            write_to_reg;
        }
    }
}
```

In order to be able to actually write to our register, we need to drive our `reset` and
`go` signals in our cocotb test:

```python
# Required for all cocotb testbenches. Included for completeness.
cocotb.start_soon(Clock(module.clk, 2, units="ns").start()) 

# Reset Calyx-generated control registers
main.reset.value = 1
aways ClockCycles(main.clk, 5) #wait a bit
module.reset.value = 0

# Start execution of control sequence
module.go.value = 1

```


[go-done]: ../../lang/ref.md#the-go-done-interface
[cocotb]: https://www.cocotb.org/
