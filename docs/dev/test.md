# Writing a Test

The `tests/verilog` folder is dedicated to Verilator correctness testing.
A `.futil` in this directory defines a new test. To get output we run the following steps:
 - Use `verilog` backend to generate a Verilog file
 - Run Verilator using `sim/testbench.cpp` as the testbench.
   - This expects a toplevel component in Futil named `main`
   - We drive the `main` component by pulsing the `clk` port on `main`
   - We simulate until the `ready` port on `main` is high.
 - Convert the generated `vcd` file to `json`
 - Use `{filename}.jq` to choose a subset of signals to test
 - Compare this generated file to `{filename.expect}`
