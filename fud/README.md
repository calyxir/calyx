# The FuTIL Driver

This is the FuTIL driver. It is a tool that automates the process
of calling FuTIL frontends, the FuTIL compiler, and any backends that may
be needed to simulate/execute a destination stage.

Working with FuTIL involves a lot of command-line tools. For example, an
incomplete yet daunting list of CLI tools used by FuTIL is:

- The Dahlia & the systolic array compilers
- FuTIL compiler and its various command line tools
- Verilator, the Verilog simulation framework used to test FuTIL-generated designs.
- Waveform viewers to see the results of simulation

`fud` aims to provide a simple interface for using these toolchains and
executing them in a pipeline.

The current documentation for fud lives [here](https://cs.capra.cornell.edu/calyx/tools/fud.html)
