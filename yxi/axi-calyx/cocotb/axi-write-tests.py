import cocotb
from cocotb.clock import Clock
from cocotbext.axi import AxiWriteBus, AxiRamWrite
from cocotb.triggers import Timer, ClockCycles
import mmap
import struct
from typing import Union, Literal, List

# TODO(nathanielnrn): If optional signals like WSTRB are not recognized,
# install cocotb-bus directly from github, as 0.2.1 has a bug


# Reads 16 elements from mmap of 16*4 bytes. Writes these elements to 16 cells in calyx defined seq_d1 mem.
@cocotb.test()
async def write_channels_happy_path(main):
    happy_data_vec = [
        1,
        2,
        4,
        8,
        16,
        32,
        64,
        128,
        256,
        512,
        1024,
        2048,
        4096,
        8192,
        16384,
        32768,
    ]
    # Expected is from hardcoded initialization of writing in axi-writes-calyx.futil
    expected = [i for i in range(16)]
    await write_axi_test_helper(main, happy_data_vec, expected)


# Adding extra data to backing mmap does not ruin reading of 16 elements and writing them correctly.
@cocotb.test()
async def write_channels_extra_mmap_data(main):
    large_data_vec = [
        1,
        2,
        4,
        8,
        16,
        32,
        64,
        128,
        256,
        512,
        1024,
        2048,
        4096,
        8192,
        16384,
        32768,
        2**32 - 1,
    ]
    expected = [i for i in range(16)]
    expected.append(2**32 - 1)
    await write_axi_test_helper(main, large_data_vec, expected)


async def write_axi_test_helper(
    module, data_vec: List[int], expected: List[int], mmap_size: int = None
):
    """Create an mmap with data of `data_vec` and use this to initialize
    a cocotb-axi-ram (write only) with this data. Assert that the data in this
    cocotb AXI Ram matches expected at the end of writing (currently expect 16 zeroes)
    correctly and matches `expected.`

    mmap_size is in bytes.

    """

    cocotb.start_soon(Clock(module.clk, 2, units="ns").start())

    # Assert reset for 5 cycles (reuqired for Calyx interfacing)
    module.reset.value = 1
    await ClockCycles(module.clk, 5)  # wait a bit
    module.reset.value = 0

    # Start the execution
    module.go.value = 1

    if mmap_size is None:
        # 4 bytes per integer
        mmap_size = len(data_vec) * 4
    # anonymous mmep for now to back axiram
    memmap = mmap.mmap(-1, mmap_size)
    axi_ram_write = AxiRamWrite(
        # NOTE: prefix should not contain the final "_"
        AxiWriteBus.from_prefix(module, "m"),
        module.clk,
        module.reset,
        # size in bytes
        size=mmap_size,
        mem=memmap,
    )

    data_vec_bytes = int_to_bytes(data_vec)
    memmap.seek(0)
    memmap.write(data_vec_bytes)
    memmap.seek(0)

    await Timer(20, "ns")
    # axi_ram_read.hexdump(0x0000, mmap_size, prefix="RAM")

    await Timer(500, "ns")
    # assert (
    #    cocotb_mem_to_ints(module.vec1_data) == expected
    # ), f"Internal memory is not {expected}, instead is {cocotb_mem_to_ints(module.vec1_data)}"

    axi_ram_mem_ints = bytes_to_int(axi_ram_write.read(0x0000, mmap_size))
    assert (
        axi_ram_mem_ints == expected
    ), f"The AXI ram: {axi_ram_mem_ints} does not contain the data in expected: {expected}."


# TODO(nathanielnrn): Decide between these and xilinx cocotb tests, refactor out
# after determining which is better


# Returns 4-byte representation of an integer
# Does not yet support unsigned, can be changed by changing to `i` as opposed to `I`.
# Not supported cause haven't yet thought about how AXI is affected
def int_to_bytes(
    integers, byteorder: Union[Literal["little"], Literal["big"]] = "little"
):
    frmt = get_format(byteorder, integers)
    return struct.pack(frmt, *integers)


# returns list of ints or a single int depending on size of bytes argument
def bytes_to_int(bytes, byteorder="little"):
    assert len(bytes) % 4 == 0, "bytes length not divisble by 4."
    frmt = get_format(byteorder, bytes)
    ints = struct.unpack(frmt, bytes)
    if len(ints) == 1:
        return ints[0]
    return list(ints)


# Returns format used by Struct, assuming we are interested in integers (so 4 bytes)
def get_format(byteorder: Union[Literal["little"], Literal["big"]], input_list):
    frmt = ""
    if byteorder == "little":
        frmt += "<"
    elif byteorder == "big":
        frmt += ">"
    else:
        raise ValueError("byteorder must be 'little' or 'big'.")

    if type(input_list) is bytes:
        assert len(input_list) % 4 == 0, "input_list length not divisble by 4."
        frmt += f"{len(input_list)//4}"
    elif type(input_list[0]) is int:
        frmt += f"{len(input_list)}"

    frmt += "I"
    return frmt


# Takes in top level cocotb memory structure and returns integers of bytes contained in it.
def cocotb_mem_to_ints(memory) -> List[int]:
    integers = list(map(lambda e: e.integer, memory.mem.value))
    # Cocotb mem.value seems to store integers in reverse order? So memory cell 0 is
    # at index -1 and memory cell n-1 is at index 0
    return integers[::-1]
