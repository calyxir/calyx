import cocotb
from cocotb.clock import Clock
from cocotbext.axi import AxiReadBus, AxiRamRead
from cocotb.triggers import Timer

# TODO(nathanielnrn): If optional signals like WSTRB are not recognized,
# install cocotb-bus directly from github, as 0.2.1 has a bug



#TODO: Go to the cocotb library and start printing out ready singals and see
# why it is none

@cocotb.test()
async def read_channels_tests(main):
    axi_ram_read = AxiRamRead(
            AxiReadBus.from_prefix(main, "m_"), main.clk, main.reset, size=2**20
    )
    axi_ram_read.hexdump(0x0000, 4, prefix="RAM")

    cocotb.start_soon(Clock(main.clk, 2, units="ns").start())

    await Timer(100, "ns")
    assert False, "Not yet implemented"
