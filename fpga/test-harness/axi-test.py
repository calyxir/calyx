import cocotb
from cocotb.clock import Clock
from cocotbext.axi import AxiBus, AxiMaster, AxiRam
from cocotc import Triggers import Timer, FallingEdge

class VectorAddTB:

    def __init__(self, dut):
        sel.dut = dut


    #set up clock of 2ns period, simulator default timestep is 1ps
    async def generate_clock(self, toplevel):
        return Clock(toplevel.ap_clk, 2, units="ns").start()
    
    async def setup_ram(self, toplevel):
        ap_rst = 1 if toplevel.ap_rst_n == 0 else 0
        #TODO(nathanielnrn): switch from hardcoded one 8 element (of 32 bits) array from
        # examples/vectorized_add.fuse.data to parameterized data
        mem_size = int(1 * 8 * 32 / 8)
        # from_prefix assumes signals of form dut.<prefix>_<signal> i.e m0_axi_RDATA
        # therefore these prefixes have to match verilog code, see kernel.xml <args>
        # and ports assigned within that.
        # In general, the index of `m<idx>_axi` just increments by 1 in fud axi generation
        self.axi_ram_a = AxiRam(AxiBus.from_prefix(toplevel, "m0_axi"), ap_rst, mem_size)
        self.axi_ram_b = AxiRam(AxiBus.from_prefix(toplevel, "m1_axi"), ap_rst, mem_size)
        self.axi_ram_sum = AxiRam(AxiBus.from_prefix(toplevel, "m2_axi"), ap_rst, mem_size)
    
        write_data_a = []
        write_data_b = []
        write_data_sum = []
        #TODO(nathanielnrn): also chage element values to be parameterized
        for i in range(mem_size):
            write_data_a.append(2 ** (i+1) - 1)
            write_data_b.append(1)
            write_data_sum.append(0)
        #TODO(nathanielnrn): bytes can only handle integers up to 256
        # can attempt to convert with
        #(list(map(labmda e: int(e).to_bytes(byte_length:int, byteorder:str,*, signed=False)
        # or look into cocotb.BinaryValue
        #TODO: define addr correctly
        self.axi_ram_a.write(addr, bytes(write_data_a))
        self.axi_ram_b.write(addr, bytes(write_data_b))
        self.axi_ram_sum.write(addr, bytes(write_data_sum))

    @cocotb.test()
    async def run_vadd_test(self, toplevel, data)):
        await cocotb.start_soon(generate_clock(toplevel))
        await setup_ram(toplevel)
        await Timer(30, unit="us")

        await FallingEdge(toplevel.ap_clk)

        #TODO: get working with data argument
        data_a = []
        data_b = []
        for i in range(mem_size):
            data_a.append(2 ** (i+1) - 1)
            data_b.append(1)
        equal = True
        #TODO: get length parametrically
        length = 1 * 8
        sum_out = self.axi_ram_sum.read(addr, length)
        assert model(data_a, data_b) == sum_out


    
    def model(self, a : List[int], b: List[int]) -> List[int]:
        """Describes the computation we expect the kernel to perform"""
        # assumes a and b are of same length for now
        return [a[i]+b[i] for i in range(len(a))]
