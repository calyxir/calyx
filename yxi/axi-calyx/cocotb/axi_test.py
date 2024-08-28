import json
import cocotb
from cocotb.clock import Clock
from cocotbext.axi import AxiBus, AxiRam, AxiLiteMaster, AxiLiteBus
from cocotb.triggers import Timer, FallingEdge, with_timeout, RisingEdge, ClockCycles
from typing import Literal, Mapping, Any, Union, List
from pathlib import Path
import os


# NOTE (nathanielnrn) cocotb-bus 0.2.1 has a bug that does not recognize optional
# signals such as WSTRB when it is capitalized. Install directly from the cocotb-bus
# github repo to fix
class KernelTB:
    def __init__(self, toplevel, data_path: Path):
        self.toplevel = toplevel
        self.data_path = data_path
        assert os.path.isfile(
            self.data_path
        ), "data_path must be a data path to a valid file"

    async def setup_control_manager(self):
        self.control_manager = AxiLiteMaster(
            AxiLiteBus.from_prefix(self.toplevel, "s_axi_control"),
            self.toplevel.clk,
            reset=self.toplevel.reset,
        )

    #Go through each mem, create an AxiRam, write data to it
    async def setup_rams(self, data: Mapping[str, Any]):
        # Create cocotb AxiRams for each `ref` memory
        rams = {}
        for mem in data.keys():
            assert not isinstance(data[mem]["data"][0], list)
            size = mem_size_in_bytes(mem, data)
            width = data_width_in_bytes(mem, data)

            # From_prefix assumes signals of form toplevel.<prefix>_<signal>
            # i.e m0_axi_RDATA.
            # These prefixes have to match verilog code. See kernel.xml <args>
            # and ports assigned within that for guidance.
            # In general, the index of `m<idx>_axi` just
            # increments by 1 in fud axi generation
            #print(f"mem is: {mem}")
            rams[mem] = AxiRam(
                AxiBus.from_prefix(self.toplevel, f"{mem}"),
                self.toplevel.clk,
                reset = self.toplevel.reset,
                # self.toplevel.ap_rst_n,
                size=size,
            )

            # NOTE: This defaults to little endian to match AxiRam defaults
            data_in_bytes = encode(data[mem]["data"], width, byteorder="little", signed = bool(data[mem]["format"]["is_signed"]))
            addr = 0x0000
            rams[mem].write(addr, data_in_bytes)

        self.rams = rams

    def get_rams(self):
        return self.rams

    async def init_toplevel(self):
        await Timer(50, "ns")
        self.toplevel.reset.value = 1
        await ClockCycles(self.toplevel.clk, 5)
        self.toplevel.reset.value = 0
        self.toplevel.go.value = 1


async def run_kernel_test(toplevel, data_path: str):
    tb = KernelTB(toplevel, Path(data_path))
    data_map = None
    with open(data_path) as f:
        data_map = json.load(f)
        f.close()
    assert data_map is not None
    await tb.setup_rams(data_map)
    await tb.setup_control_manager()
    #print(data_map)

    
    # set up clock of 2ns period, simulator default timestep is 1ps
    cocotb.start_soon(Clock(toplevel.clk, 2, units="ns").start())
    await tb.init_toplevel()
    await Timer(100, "ns")
    await FallingEdge(toplevel.clk)

    # Finish when ap_done is high or 100 us of simulation have passed.
    timeout = 5000
    # Base addresses for memories
    # The od verilog wrapper seemed to be ok with base addresses of 0x0000
    # for every memory, so trying that here.
    # Xilinx spec has the first argument offset at 0x0010
    # Note this differs from the old verilog testrunner because we assume no
    # timeout argument with the new calyx wrapper.
    register_offset = 0x0010
    for mem in data_map.keys():
        await tb.control_manager.write(register_offset, encode([0x0],4))
        register_offset += 4
        await tb.control_manager.write(register_offset, encode([0x0],4))
        register_offset += 4
    #Assert ap_start by writing 1 to 0x0000
    await tb.control_manager.write(0x0000, encode([0x1],1))
    await with_timeout(RisingEdge(toplevel.done), timeout, "us")



    
    # Get data from ram
    mems: list[str] = list(data_map.keys())
    rams = tb.get_rams()
    post = {}
    for mem in mems:
        addr = 0x000
        size = mem_size_in_bytes(mem, data_map)
        post_execution = rams[mem].read(addr, size)
        width = data_width_in_bytes(mem, data_map)
        post_execution = decode(post_execution, width)
        post.update({mem:{"data" : post_execution}})
        post[mem]["format"] = data_map[mem]["format"]
    # post = {"memories": post}

    print("Output:\n" + json.dumps(post, indent = 2))


def mem_size_in_bytes(mem: str, data):
    """Returns size of memory within data in bytes"""
    width = data_width_in_bytes(mem, data)
    length = len(data[mem]["data"]) * width
    return length


def data_width_in_bytes(mem: str, data):
    """Returns data width of mem in bytes"""
    assert mem in data, "mem must be a key in data"
    width = data[mem]["format"]["width"] // 8
    if data[mem]["format"]["width"] % 8 != 0:
        width += 1
    return width


# AxiRam assumes little bytorder, hence the defaults
def decode(
    b: bytes,
    width: int,
    byteorder: Union[Literal["little"], Literal["big"]] = "little",
    signed=False,
):
    """Return the list of `ints` corresponding to value in `b` based on
    encoding of `width` bytes
    For example, `decode('b\x00\x00\x00\04', 4)` returns `[4]`
    """
    assert len(b) % width == 0, "Mismatch between bytes length and width"
    to_return = []
    for i in range(len(b) // width):
        start = i * width
        end = start + width
        to_return.append(
            int.from_bytes(b[start:end], byteorder=byteorder, signed=signed)
        )
    return to_return


def encode(
    lst: List[int],
    width,
    byteorder: Union[Literal["little"], Literal["big"]] = "little",
    signed: bool = False
    ) -> bytes:
    """Return the `width`-wide byte representation of lst with byteorder"""
    return b''.join(i.to_bytes(width, byteorder, signed=signed) for i in lst)
