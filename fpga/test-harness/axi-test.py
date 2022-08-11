import json
import cocotb
from cocotb.clock import Clock
from cocotbext.axi import AxiBus, AxiRam, AxiLiteMaster, AxiLiteBus, AxiLiteRam
from cocotb.triggers import Timer, FallingEdge, RisingEdge
from typing import Mapping, List, Any
from pathlib import Path
import os


class VectorAddTB:
    def __init__(self, toplevel):
        self.toplevel = toplevel

    async def setup_rams(self, data: Mapping[str, Any]):
        # Create cocotb AxiRams
        rams = {}
        addr = 0x0000
        for i, mem in enumerate(data.keys()):
            # vectorized add should only use 1 dimensional list
            assert not isinstance(data[mem]["data"][0], list)
            mem_size = self.mem_size(mem, data)
            width = self.data_width(mem, data)
            # from_prefix assumes signals of form dut.<prefix>_<signal> i.e m0_axi_RDATA
            # therefore these prefixes have to match verilog code, see kernel.xml <args>
            # and ports assigned within that.
            # In general, the index of `m<idx>_axi` just
            # increments by 1 in fud axi generation

            # TODO: need to fix toplevel.ap_rst_n
            rams[mem] = AxiRam(
                AxiBus.from_prefix(self.toplevel, f"m{i}_axi"),
                self.toplevel.ap_clk,
                # self.toplevel.ap_rst_n,
                size=mem_size,
            )
            # TODO(nathanielnrn): bytes can only handle integers up to 256
            # can attempt to convert with
            # (list(map(
            # labmda e: int(e).to_bytes(byte_length:int, byteorder:str,*, signed=False
            # )
            # or look into cocotb.BinaryValue

            # TODO: check if this addr definition is correct and works

            # This corresponds with waveform of numbers, in practice,
            # Each address is +4096 from last

            data_in_bytes = [
                i.to_bytes(width, byteorder="big") for i in data[mem]["data"]
            ]
            data_in_bytes = b"".join(data_in_bytes)
            rams[mem].write(addr, data_in_bytes)
            # TODO: This is now 0 every time?? Because the rams created are
            # not big enough? Even though size of bram in wave is also shrunk?

            # TODO: remove 3 lines
            # addr = addr + 0x1000
            # in_ram = [b for b in rams[mem].read(addr, mem_size)]
            # print(f"{mem} ram currently is: {in_ram}")
        self.rams = rams

    def model(self, a: List[int], b: List[int]) -> List[int]:
        """Describes the computation we expect the kernel to perform"""
        # assumes a and b are of same length for now
        assert len(a) == len(b), "a and b must be of same length"
        return [a[i] + b[i] for i in range(len(a))]

    def mem_size(self, mem: str, data):
        """Returns size of memory within data in bytes"""
        width = self.data_width(mem, data)
        length = len(data[mem]["data"]) * width
        return length

    def data_width(self, mem: str, data):
        """Returns data width of mem in bytes"""
        assert mem in data, "mem must be a key in data"
        width = data[mem]["format"]["width"] // 8
        if data[mem]["format"]["width"] % 8 != 0:
            width += 1
        return width

    def get_rams(self):
        return self.rams

    def drive_controls(self):
        tl = self.toplevel
        addr_signal = 16
        wdata_signal = 0

        while True:
            tl.s_axi_control_AWADDR = addr_signal
            if tl.s_axi_control_AWVALID.value & tl.s_axi_control_AWREADY.value:
                addr_signal += 4


    async def reset(self):
        await Timer(100, "ns")
        self.toplevel.ap_rst_n.value = 0
        await Timer(100, "ns")
        self.toplevel.ap_rst_n.value = 1
        await Timer(100, "ns")

    def get_control(self):
        return AxiLiteMaster(
            AxiLiteBus.from_prefix(self.toplevel, "s_axi_control"), self.toplevel.ap_clk
        )

    def get_mem(self, prefix : str):
        return AxiMaster(
            AxiBus.from_prefix(self.toplevel, f"{prefix}_axi", self.toplevel.ap_clk)
        )


@cocotb.test(skip=True)
async def one_transaction(toplevel):
    cocotb.start_soon(Clock(toplevel.ap_clk, 2, units="ns").start())
    vadd_tb = VectorAddTB(toplevel)
    control = vadd_tb.get_control()
    data = bytes([1])
    addr = 0x0000

    await Timer(1, "us")
    toplevel.ap_rst_n.value = 0
    await Timer(1, "us")
    toplevel.ap_rst_n.value = 1
    await Timer(1, "us")
    await control.write(addr, data)
    await Timer(10, "ns")
    
    out = await control.read(addr,1)
    print(out)
    assert int.from_bytes(out[1], byteorder='big') == 1


@cocotb.test(skip=True)
async def run_vadd_test(toplevel):
    data_path = Path("../../examples/dahlia/vectorized-add.fuse.data")
    assert os.path.isfile(data_path), "data_path must be a data path to a valid file"
    data = None
    with open(data_path) as f:
        data = json.load(f)

    assert data is not None
    vadd_tb = VectorAddTB(toplevel)

    # set up clock of 2ns period, simulator default timestep is 1ps
    cocotb.start_soon(Clock(toplevel.ap_clk, 2, units="ns").start())
    await vadd_tb.setup_rams(data)
    # await Timer(0.0001,"us")
    await Timer(30, "us")
    await FallingEdge(toplevel.ap_clk)

    mems: list[str] = list(data.keys())
    # We assume last memory is the output
    mem_length = vadd_tb.mem_size(mems[-1], data)
    # TODO: Make sure this is correct
    rams = vadd_tb.get_rams()
    # Find correct value of addr
    addr = 0x000
    # addr = (len(rams) - 1) * int("0x1000", 0)

    # byte literal form
    sum_out = rams[mems[-1]].read(addr, mem_length)
    sum_out = decode(sum_out, vadd_tb.data_width("Sum0", data))
    print(f"sum_out is {sum_out}")
    # assumes first two mems are inputs
    assert vadd_tb.model(data[mems[0]]["data"], data[mems[1]]["data"]) == sum_out

@cocotb.test()
async def manual_vadd(toplevel):

    vadd_tb = VectorAddTB(toplevel)
    cocotb.start_soon(Clock(toplevel.ap_clk, 2, units="ns").start())
    #m0 = vadd_tb.get_mem("m0")

    await vadd_tb.reset()
    toplevel.ap_start.value = 1

    A0 = [1,3,7,15,31,63,127,255]
    B0 = [1,1,1,1,1,1,1,1]
    Sum0 = [0,0,0,0,0,0,0,0]

    m0 = toplevel.inst_mem_controller_axi_0
    m1 = toplevel.inst_mem_controller_axi_1
    m2 = toplevel.inst_mem_controller_axi_2

    data_width = 32
    elements = len(Sum0)
    mem_size = data_width * elements // 8
    #TODO: remove
    mem_size = 1024
    ram = AxiLiteRam(AxiLiteBus.from_prefix(toplevel, "m2_axi"),toplevel.ap_clk, size=mem_size)
    sum0_in_bytes = [i.to_bytes(32 // 8, byteorder="big") for i in Sum0]
    sum0_in_bytes = b"".join(sum0_in_bytes)
    ram.write(0, sum0_in_bytes)

    async def write_to_bram(data : list[int], mem):
        await Timer(200,"ns")
        while mem.copy_done.value != 1 and mem.read_txn_count.value != 8:
            #TODO: make dynamic using vadd_tb.data_width(mem, data)
            data_width = 32
            mem.RDATA.value = data[mem.read_txn_count.value] * 2 ** (mem.read_txn_count.value * data_width)
            mem.RVALID.value = 1
            await RisingEdge(toplevel.ap_clk)
            mem.ARREADY.value = 1

    async def read_from_bram(addr: int, mem):
        mem.SEND_TO_HOST.value = 1
        #while mem.send_done.value != 1:
        #    #AW
        #    mem.AWREADY.value = 1
        #    #W
        #    mem.WREADY.value = 1
        #    #B
        #    mem.BVALID.value = 1
        #    await RisingEdge(toplevel.ap_clk)
        

    
    #TODO: Why does this need an await?
    #We must copy __all__ memories for kernel to correctly recognize it has finished
    #copying all 
    await cocotb.start(write_to_bram(A0, m0))
    await cocotb.start(write_to_bram(B0, m1))
    await cocotb.start(write_to_bram(Sum0, m2))

    await Timer(500, 'ns')
    await cocotb.start(read_from_bram(0, m2))

   # await Timer(200,"ns")
   # #while copying from host to internal bram
   # while m0.copy_done.value != 1:
   #     #TODO: make dynamic using vadd_tb.data_width(mem, data)
   #     data_width = 32
   #     #Equivalent to bit shifted value in A0
   #     m0.RDATA.value = A0[m0.read_txn_count.value] * 2 ** (m0.read_txn_count.value * data_width)
   #     m0.RVALID.value = 1
   #     await RisingEdge(toplevel.ap_clk)
   #     #m0.RREADY.value = 1

   #     m0.ARREADY.value = 1
    await Timer(5,"us")

    #out = [int(str(n),2) for n in m0.bram.ram_core.value]
    out = m0.bram.ram_core.value
    addr = 0
    length = 8 * 4
    print(decode(ram.read(addr, length),4, byteorder='little'))
    assert False


def decode(b: bytes, width: int, byteorder="big", signed=False):
    """Return the list of `ints` corresponding to value in `b` based on
    encoding of `width` bytes
    For example, `decode('b\x00\x00\x00\04', 4)` returns `[4]`
    """
    assert len(b) % width == 0, "Mismatch between bytes length and width"
    to_return = []
    for i in range(len(b) // width):
        to_return.append(
            int.from_bytes(
                b[i * width : (i + 1) * width], byteorder=byteorder, signed=signed
            )
        )
    return to_return

def encode(lst:list[int], width, byteorder="big"):
    """Return the `width`-wide byte representation of lst with byteorder"""
    return [i.to_bytes(width, byteorder) for i in lst]


