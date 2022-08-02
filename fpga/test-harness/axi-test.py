import json
import cocotb
from cocotb.clock import Clock
from cocotbext.axi import AxiBus, AxiMaster, AxiRam
from cocotb import Triggers, Timer, FallingEdge
from typing import Mapping, List, Any
from pathlib import Path
import os


class VectorAddTB:

    # def __init__(self, toplevel):
    # self.toplevel = toplevel

    # set up clock of 2ns period, simulator default timestep is 1ps
    async def generate_clock(self, toplevel):
        return Clock(toplevel.ap_clk, 2, units="ns").start()

    async def setup_rams(self, toplevel, data: Mapping[str, Any]):
        # Create cocotb AxiRams
        rams = {}
        addr = int('0x0000', 0)
        for i, mem in enumerate(data.keys()):
            # vectorized add should only use 1 dimensional list
            assert not isinstance(data[mem]["data"][0], list)
            mem_size = self._mem_size(mem, data)
            # from_prefix assumes signals of form dut.<prefix>_<signal> i.e m0_axi_RDATA
            # therefore these prefixes have to match verilog code, see kernel.xml <args>
            # and ports assigned within that.
            # In general, the index of `m<idx>_axi` just increments by 1 in fud axi generation
            rams[mem] = AxiRam(
                AxiBus.from_prefix(toplevel, f"m{i}_axi"), toplevel.ap_rst_n, mem_size
            )
            # TODO(nathanielnrn): bytes can only handle integers up to 256
            # can attempt to convert with
            # (list(map(labmda e: int(e).to_bytes(byte_length:int, byteorder:str,*, signed=False)
            # or look into cocotb.BinaryValue
            # TODO: check if this addr definition is correct and works

            # This corresponds with waveform form of numbers, in practice,
            # Each address is +4096 from last
            addr = addr + int('0x1000', 0)
            rams[mem].write(addr, bytes(data[mem]["data"]))
        self.rams = rams

    @cocotb.test()
    async def run_vadd_test(self, toplevel, data):
        data_path = Path('../../examples/dahlia/vectorized-add.fuse.data')
        assert os.path.isfile(data_path), "data_path must be a data path to a valid file"
        with open(data_path) as f:
            data = json.load(f)

        await cocotb.start_soon(self.generate_clock(toplevel))
        await self.setup_ram(toplevel, data)
        await Timer(30, unit="us")

        await FallingEdge(toplevel.ap_clk)

        mems: list[str] = data.keys()
        # We assume last memory is the output
        length = self._mem_size(mems[-1], data)
        #TODO: Define addr correctly
        addr = (len(self.rams) - 1) * int('0x1000', 0)
        sum_out = self.rams[mems[-1]].read(addr, length)
        # assumes first two mems are inputs
        assert self.model(data[mems[0]], data[mems[1]]) == sum_out

    def model(self, a: List[int], b: List[int]) -> List[int]:
        """Describes the computation we expect the kernel to perform"""
        # assumes a and b are of same length for now
        assert len(a) == len(b), "a and b must be of same length"
        return [a[i] + b[i] for i in range(len(a))]

    def _mem_size(self, mem: str, data):
        """Returns size of memory within data in bytes"""
        assert mem in data, "mem must be a key in data"
        width = data["format"]["width"] // 8
        if data["format"]["width"] % 8 != 0 :
            width += 1

        length = len(data[mem]["data"]) * width
        return length
