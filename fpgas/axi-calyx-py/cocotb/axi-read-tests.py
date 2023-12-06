import cocotb
from cocotb.clock import Clock
from cocotbext.axi import AxiReadBus, AxiRamRead
from cocotb.triggers import Timer
import mmap

# TODO(nathanielnrn): If optional signals like WSTRB are not recognized,
# install cocotb-bus directly from github, as 0.2.1 has a bug




@cocotb.test()
async def read_channels_tests(main):

    memmap = mmap.mmap(-1, 128)

    axi_ram_read = AxiRamRead(
            #NOTE: prefix should not contain the final "_"
            AxiReadBus.from_prefix(main, "m"), main.clk, main.reset, size=128, mem=memmap
    )

    memmap.seek(0)
    memmap.write(b"xyz")
    memmap.seek(0)
    print(f"memmap read: {memmap.read(3).decode('utf-8')}")

    await Timer(20, "ns")
    #main.m_ap_start.value = 1
    axi_ram_read.hexdump(0x0000, 4, prefix="RAM")

    cocotb.start_soon(Clock(main.clk, 2, units="ns").start())

    await Timer(1000, "ns")
    print(f"vec1_data: {main.vec1_data}")
    assert main.vec1_data.mem[0] == b"x", "Axi channel failed"
