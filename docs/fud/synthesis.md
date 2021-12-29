# Xilinx Toolchain

> Working with vendor EDA toolchains is never a fun experience. Something will
> almost certainly go wrong. If you're at Cornell, you can at least avoid
> installing the tools yourself by using our lab servers, [Gorgonzola][] or [Havarti][].

`fud` can interact with the Xilinx tools ([Vivado][], [Vivado HLS][vhls], and [Vitis][]). There are three main things it can do:

* Synthesize Calyx-generated RTL designs to collect area and resource estimates.
* Compile [Dahlia][] programs via C++ and [Vivado HLS][vhls] for comparison with the Calyx backend.
* Compile Calyx programs for *actual execution* in Xilinx emulation modes or on real FPGA hardware.

You can set `fud` up to use either a local installation of the Xilinx tools or one on a remote server, via SSH.

## Synthesis Only

The simplest way to use the Xilinx tools is to synthesize RTL or HLS designs to collect statistics about them.
This route will not produce actual, runnable executables; see the next section for that.

### Set Up

To set up to **invoke the Xilinx tools over SSH**, first tell `fud` your username and hostname for the server:

    # Vivado
    fud config stages.synth-verilog.ssh_host <hostname>
    fud config stages.synth-verilog.ssh_username <username>

    # Vivado HLS
    fud config stages.vivado-hls.ssh_host <hostname>
    fud config stages.vivado-hls.ssh_username <username>

The server must have `vivado` and `vivado_hls` available on the remote machine's path. (If you need the executable names to be something else, please file an issue.)

To instead **invoke the Xilinx tools locally**, just let `fud` run the `vivado` and `vivado_hls` commands.
You can optionally tell `fud` where these commands exist on your machine:

    fud config stages.synth-verilog.exec <path> # update vivado path
    fud config stages.vivado-hls.exec <path> # update vivado_hls path

### Run

To run the entire toolchain and extract statistics from RTL synthesis, use the `resource-estimate` target state.
For example:

    fud e --to resource-estimate examples/futil/dot-product.futil

To instead obtain the raw synthesis results, use `synth-files`.

To run the analogous toolchain for Dahlia programs via HLS, use the `hls-estimate` target state:

    fud e --to hls-estimate examples/dahlia/dot-product.fuse

There is also an `hls-files` state for the raw results of Vivado HLS.

## Emulation and Execution

`fud` can also compile Calyx programs for actual execution, either in the Xilinx toolchain's emulation modes or for running on a physical FPGA.
This route involves generating an [AXI][] interface wrapper for the Calyx program and invoking it using [XRT][]'s OpenCL interface.

### Set Up

As above, you can invoke the Xilinx toolchain locally or remotely, via SSH.
To set up SSH execution, you can edit your `config.toml` to add settings like this:

    [stages.xclbin]
    ssh_host = "havarti"
    ssh_username = "als485"
    remote = true

To use local execution, just leave off the `remote = true` line.

You can also set the Xilinx mode and target device:

    [stages.xclbin]
    mode = "hw_emu"
    device = "xilinx_u50_gen3x16_xdma_201920_3"

The options for `mode` are `hw_emu` (simulation) and `hw` (on-FPGA execution).
The device string above is for the [Alveo U50][u50] card, which we have at Cornell, but I honestly don't know how you're supposed to find the right string for a different FPGA target.
Hopefully someone will figure this out and document it in the future.

### Compile

The first step in the Xilinx toolchain is to generate [an `xclbin` executable file][xclbin].
Here's an example of going all the way from a Calyx program to that:

    fud e --to xclbin examples/futil/dot-product.futil

On our machines, compiling even a simple example like the above for simulation takes about 2 minutes, end to end.

### How it Works

The first step is to generate input files.
We need to generate:

* The RTL for the design itself, using the compile command-line flags `-b verilog --synthesis -p external`. We name this file `main.sv`.
* A Verilog interface wrapper, using `XilinxInterfaceBackend`, via `-b xilinx`. We call this `toplevel.v`.
* An XML document describing the interface, using `XilinxXmlBackend`, via `-b xilinx-xml`. This file gets named `kernel.xml`.

We also use [a static Tcl script, `gen_xo.tcl`,][gen_xo] to drive the Xilinx tools.
The `fud` driver gathers these files together in a sandbox directory.
Then, the Xilinx toolchain's first step is to compile the Verilog to a `.xo` file, which is a Xilinx analog of a `.o` object file.
The Vivado command line looks roughly like this:

    vivado -mode batch -source gen_xo.tcl -tclargs xclbin/kernel.xo kernel hw_emu xilinx_u50_gen3x16_xdma_201920_3

Those arguments after `-tclargs`, unsurprisingly, get passed to [`gen_xo.tcl`][gen_xo].

Then, we take this `.xo` and turn it into an [`.xclbin`][xclbin], in a step that is Xilinx's analog of "linking" an executable.
This step uses the `v++` tool, with a command line that looks like this:

    v++ -g -t hw_emu --platform xilinx_u50_gen3x16_xdma_201920_3 --save-temps --profile.data all:all:all --profile.exec all:all:all -lo xclbin/kernel.xclbin xclbin/kernel.xo

[vivado]: https://www.xilinx.com/products/design-tools/vivado.html
[vhls]: https://www.xilinx.com/products/design-tools/vivado/integration/esl-design.html
[gorgonzola]: https://capra.cs.cornell.edu/private/gorgonzola.html
[havarti]: https://capra.cs.cornell.edu/private/havarti.html
[vitis]: https://www.xilinx.com/products/design-tools/vitis/vitis-platform.html
[dahlia]: https://capra.cs.cornell.edu/dahlia/
[axi]: https://en.wikipedia.org/wiki/Advanced_eXtensible_Interface
[xrt]: https://xilinx.github.io/XRT/
[xclbin]: https://xilinx.github.io/XRT/2021.2/html/formats.html#xclbin
[gen_xo]: https://github.com/cucapra/calyx/blob/master/fud/bitstream/gen_xo.tcl
[u50]: https://www.xilinx.com/products/boards-and-kits/alveo/u50.html
