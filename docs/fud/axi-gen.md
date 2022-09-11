# Axi generation

Calyx currently generates a fairly complex AXI interface that can be daunting
to deal with if confronting for the first time.
The following is an overview of how the generation occurs
and how the output AXI interface behaves as of 2022-9-11.

In general, when `fud` is asked to create an [`.xclbin` file][xclbin] a `kernel.xml`,
`main.sv`, and `toplevel.v` are created as intermediate steps for [xilinx tools][xilinx_tools]
to properly work.

`main.sv` contains the SystemVerilog needed for our computations to perform
correctly. It is implemented as an FSM that derives from the original Calyx program.
These files are very hard to read and not really conducive to being examined directly.
[`kernel.xml`][kernel_xml] defines register maps and ports of our
toplevel xilinx tools needs. `toplevel.v` wraps our computation kernel and contains
the AXI interface for each memory define in a calyx program. 

## Toplevel

Our toplevel is generated through files in `src/backend/xilinx/`.

### AXI memory controller
Here, a calyx program is queried and all of its sub components are extracted.
Most importantly, memories are extracted and separate AXI interfaces are created for each
one (meaning, there is no shared bus between memories). Each memory has its own
(single port) bram which writes data taken from an `mi_axi_RDATA` wire where i is the index of
the memory. Eventually the brams are read and fed into the
computation kernel of `main.sv`, which outputs results directly into the relevant
memories as defined in the original Calyx program.

For each memory we dynamically determine necesarry address and data widths and sizes.
However there is always the possiblity that something is hardcoded as a remnant
from previous versions of our AXI generation. If something is hardcoded where it shouldn't
be please open an [issue][issues].

AXI memory controllers are constructed as (full) [AXI4 managers][signals] that lack a small amount
of functionality. For example, xPROT signals are not currently supported.
Additionally, things like bursting are not currently supported, but should be
easy to implement due to the existing infrastructure and generation.


A list of current signals that are hardcoded follows:

* `xLEN` is set to 0, correlating to aburt length of 1.
* `xBURST` is set to 01, correlating to INCR type of bursts.
* `xSIZE` is set to the width of the data we are using in bytes.
* `xPROT` is not generated, and is therefore not supported.
* `xLOCK` is not generated, defaulting to 0 (normal accesses).
* `xCACHE` is not generated, making accesses non-modifiable, non-bufferable.
* `xQOS` is not generated. See [QoS signaling](https://developer.arm.com/documentation/ihi0022/e/AMBA-AXI3-and-AXI4-Protocol-Specification/AXI4-Additional-Signaling/QoS-signaling/QoS-interface-signals?lang=en).
* `xREGION` is not generated. See [Multiple region signaling](https://developer.arm.com/documentation/ihi0022/e/AMBA-AXI3-and-AXI4-Protocol-Specification/AXI4-Additional-Signaling/Multiple-region-signaling/Additional-interface-signals?lang=en).
* No [low power signals](https://developer.arm.com/documentation/ihi0022/e/AMBA-AXI3-and-AXI4-Protocol-Specification/Signal-Descriptions/Low-power-interface-signals?lang=en) are generated.

### Subordinate AXI control controller
In addition to our manager memory controllers, a subordinate controller for
our control module is also generated. This module is responsible for signaling
our computational kernel to start working, as well as calculating the correct
base addresses to use for our memory controllers. Things like address and
data widths are hard coded at the moment. It is suspected that this hardcoding
is okay for the types of programs we generate. But more work needs to be done to see
if our control structure works for arbitrary programs or needs to be changed to
allow this.


[pynq]: https://github.com/Xilinx/PYNQ
[xclbin]: https://xilinx.github.io/XRT/2021.2/html/formats.html#xclbin
[xilinx_tools]: https://github.com/cucapra/calyx/blob/master/docs/fud/xilinx.md
[kernel_xml]: https://docs.xilinx.com/r/en-US/ug1393-vitis-application-acceleration/RTL-Kernel-XML-File
[external]: https://docs.calyxir.org/lang/attributes.html?highlight=external#external
[issues]: https://github.com/cucapra/calyx/issues
[signals]: https://developer.arm.com/documentation/ihi0022/e/AMBA-AXI3-and-AXI4-Protocol-Specification/Signal-Descriptions?lang=en
