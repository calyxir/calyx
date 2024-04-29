# AXI Interface Generation


In order to run programs on FPGAs, fud is capable of utilizing
Calyx to generate a fairly complex AXI interface that can be daunting
to deal with if confronting for the first time.
The following is an overview of how the generation occurs
and how the fud-generated AXI interface behaves as of 2022-9-11.

In general, when `fud` is asked to create an [`.xclbin` file][xclbin] a `kernel.xml`,
`main.sv`, and `toplevel.v` are created as intermediate files required for our
[Xilinx tools][xilinx_tools] to properly work.


`main.sv` contains the SystemVerilog implementation of the Calyx program
we are interested in executing on an FPGA.
`toplevel.v` wraps our SystemVerilog implementation and contains
the AXI interface for each memory marked [`@external`][external] in a Calyx program.
Our `toplevel.v` and [`kernel.xml`][kernel_xml] is what our Xilinx tools interface with.
Our toplevel adheres to the Xilinx [kernel interface requirements][kernel_requirements].
`kernel.xml` defines register maps and ports of our
toplevel module used by our Xilinx tools.

For more information on the generation of the files mentioned above see [how the Xilinx Toolchain works][xilinx_how]

## Toplevel

Our [toplevel][toplevel] is generated through files in `src/backend/xilinx/`.

### AXI memory controller
Here, the [toplevel][toplevel] component of a Calyx program is queried and
memories marked [`@external`][external] are turned into AXI buses.
To note, separate AXI interfaces are created for each memory
(meaning, there is no shared bus between memories). Each memory has its own
(single port) BRAM which writes data taken from an `mi_axi_RDATA` wire where `i` is the index of
the memory. Eventually the BRAMs are read and fed into the
computation kernel of `main.sv`, which outputs results directly into the relevant
memories as defined in the original Calyx program.
Addresses, data widths, and sizes are determined from cell declarations.

> There is always the possiblity that something is hardcoded as a remnant
> of previous versions of our AXI generation. If something is hardcoded where it shouldn't
> be please open an [issue][issues].

AXI memory controllers are constructed as (full) [AXI4 managers][signals] that lack a small amount
of functionality. For example, [xPROT signals][access_protection] are not currently supported.
Additionally, things like [bursting][bursting] are not currently supported, but should be
easy to implement due to the existing infrastructure and generation tools.


A list of current signals that are hardcoded follows:

* `xLEN` is set to 0, corresponding to a burst length of 1.
* `xBURST` is set to 01, corresponding to INCR type of bursts.
* `xSIZE` is set to the width of the data we are using in bytes.
* `xPROT` is not generated, and is therefore not supported.
* `xLOCK` is not generated, defaulting to 0 (normal accesses).
* `xCACHE` is not generated, making accesses non-modifiable, non-bufferable.
* `xQOS` is not generated. See [QoS signaling](https://developer.arm.com/documentation/ihi0022/e/AMBA-AXI3-and-AXI4-Protocol-Specification/AXI4-Additional-Signaling/QoS-signaling/QoS-interface-signals?lang=en).
* `xREGION` is not generated. See [Multiple region signaling](https://developer.arm.com/documentation/ihi0022/e/AMBA-AXI3-and-AXI4-Protocol-Specification/AXI4-Additional-Signaling/Multiple-region-signaling/Additional-interface-signals?lang=en).
* No [low power signals](https://developer.arm.com/documentation/ihi0022/e/AMBA-AXI3-and-AXI4-Protocol-Specification/Signal-Descriptions/Low-power-interface-signals?lang=en) are generated.

### Subordinate AXI control controller
In addition to our manager memory-controllers, a subordinate interface,
connected to our control module is also generated.
This module is responsible for signaling
our computational kernel to start working, as well as calculating the correct
base addresses to use for our memory controllers. Things like addresses and
data widths are hard coded at the moment. It is suspected that this hardcoding
is okay for the types of programs we generate. But more work needs to be done to see
if our control structure works for arbitrary programs or needs to be changed to
allow this.


[pynq]: https://github.com/Xilinx/PYNQ
[xclbin]: https://xilinx.github.io/XRT/2021.2/html/formats.html#xclbin
[xilinx_tools]: https://github.com/calyxir/calyx/blob/master/docs/fud/xilinx.md
[kernel_xml]: https://docs.xilinx.com/r/en-US/ug1393-vitis-application-acceleration/RTL-Kernel-XML-File
[external]: https://docs.calyxir.org/lang/attributes.html?highlight=external#external
[issues]: https://github.com/calyxir/calyx/issues
[signals]: https://developer.arm.com/documentation/ihi0022/e/AMBA-AXI3-and-AXI4-Protocol-Specification/Signal-Descriptions?lang=en
[bursting]: https://developer.arm.com/documentation/ihi0022/e/AMBA-AXI3-and-AXI4-Protocol-Specification/Single-Interface-Requirements/Transaction-structure/Address-structure?lang=en
[access_protection]: https://developer.arm.com/documentation/ihi0022/e/AMBA-AXI3-and-AXI4-Protocol-Specification/Transaction-Attributes/Access-permissions?lang=en
[toplevel]: https://docs.calyxir.org/lang/attributes.html?highlight=toplevel#toplevel
[xilinx_how]: https://docs.calyxir.org/running-calyx/fud/xilinx.html?highlight=synthesis#how-it-works
[kernel_requirements]: https://docs.xilinx.com/r/en-US/ug1393-vitis-application-acceleration/Kernel-Interface-Requirements
