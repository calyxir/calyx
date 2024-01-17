# Interfacing with Calyx Programs

To run RTL designs created from Calyx programs, top-level `reset`, `go`, and `done`
signals must be interfaced with correctly.
Interfacing with RTL designs in this way becomes relevant when writing harnesses/testbenches
to execute programs created with Calyx.

Namely, the client for a Calyx top-level module must:
1. Assert the `reset` signal and then deassert it, to initialize the state inside
control registers correctly.
2. Assert the `go` signal, and keep it asserted as long as the module is running.
3. Wait for the `done` signal to be asserted while keeping `go` high. Deasserting
the `go` signal before a component deasserts its `done` signal will lead to
[undefined behavior][go-done].

Asserting the `reset` and `go` signals in this order is important. Otherwise the top-level
component will begin running with garbage data inside of control registers.


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
await ClockCycles(main.clk, 5) #wait a bit
module.reset.value = 0

# Start execution of control sequence
module.go.value = 1

#At this point our Calyx program is done
await RisingEdge(main.done)
```

[go-done]: ../lang/ref.md#the-go-done-interface
[cocotb]: https://www.cocotb.org/
