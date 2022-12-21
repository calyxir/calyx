import json
import cocotb
from cocotb.clock import Clock
from cocotbext.axi import AxiBus, AxiRam
from cocotb.triggers import Timer, FallingEdge, with_timeout
from typing import Literal, Mapping, Any, Union
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
        # self.expect_path = expect_path
        # assert os.path.isfile(
        #    self.expect_path
        # ), "data_path must be a data path to a valid file"

    async def setup_rams(self, data: Mapping[str, Any]):
        # Create cocotb AxiRams
        rams = {}
        for i, mem in enumerate(data.keys()):
            assert not isinstance(data[mem]["data"][0], list)
            size = mem_size(mem, data)
            width = data_width(mem, data)

            # From_prefix assumes signals of form toplevel.<prefix>_<signal>
            # i.e m0_axi_RDATA.
            # These prefixes have to match verilog code. See kernel.xml <args>
            # and ports assigned within that for guidance.
            # In general, the index of `m<idx>_axi` just
            # increments by 1 in fud axi generation
            rams[mem] = AxiRam(
                AxiBus.from_prefix(self.toplevel, f"m{i}_axi"),
                self.toplevel.ap_clk,
                # XXX (nathanielnrn): no easy way to invert ap_rst_n signal
                # through cocotb
                # self.toplevel.ap_rst_n,
                size=size,
            )

            # NOTE: This defaults to little endian to match AxiRam defaults
            data_in_bytes = encode(data[mem]["data"], width)
            addr = 0x0000
            for byte_data in data_in_bytes:
                rams[mem].write(addr, byte_data)
                addr += width

        self.rams = rams

    def get_rams(self):
        return self.rams

    async def reset(self):
        await Timer(50, "ns")
        self.toplevel.ap_rst_n.value = 0
        await Timer(50, "ns")
        self.toplevel.ap_rst_n.value = 1


async def run_kernel_test(toplevel, data_path: str):
    # XXX (nathanielnrn): This only works if data passed in is less than 64 bytes
    # (512 bits) because the AxiRam isn't correctly writing to our generated
    # verilog. Speicfically, RDATA is a dump of all of the ram data, seemingly
    # regardless of ARADDR. When too much dta is passed in they are simply dropped
    tb = KernelTB(toplevel, Path(data_path))
    await tb.reset()

    data = None
    with open(data_path) as f:
        data = json.load(f)
        f.close()
    assert data is not None

    # set up clock of 2ns period, simulator default timestep is 1ps
    cocotb.start_soon(Clock(toplevel.ap_clk, 2, units="ns").start())
    await tb.setup_rams(data)
    await Timer(100, "ns")
    await FallingEdge(toplevel.ap_clk)

    toplevel.ap_start.value = 1

    # Get data from ram
    mems: list[str] = list(data.keys())
    rams = tb.get_rams()

    # Finish when ap_done is high or 100 us of simulation have passed.
    timeout = 100
    await with_timeout(FallingEdge(toplevel.ap_done), timeout, "us")

    post = {}
    for mem in mems:
        addr = 0x000
        size = mem_size(mem, data)
        post_execution = rams[mem].read(addr, size)
        width = data_width(mem, data)
        post_execution = decode(post_execution, width)
        post.update({mem: post_execution})
    post = {"memories": post}

    print("Output:" + json.dumps(post))


def mem_size(mem: str, data):
    """Returns size of memory within data in bytes"""
    width = data_width(mem, data)
    length = len(data[mem]["data"]) * width
    return length


def data_width(mem: str, data):
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
    lst: list[int],
    width,
    byteorder: Union[Literal["little"], Literal["big"]] = "little",
):
    """Return the `width`-wide byte representation of lst with byteorder"""
    return [i.to_bytes(width, byteorder) for i in lst]
