import cocotb
from cocotb.clock import Clock
from cocotbext.axi import AxiReadBus, AxiRamRead
from cocotb.triggers import Timer, ClockCycles
import mmap
import struct
from typing import Union, Literal, List

# TODO(nathanielnrn): If optional signals like WSTRB are not recognized,
# install cocotb-bus directly from github, as 0.2.1 has a bug


# Reads 16 elements from mmap of 16*4 bytes. Writes these elements to 16 cells in calyx defined seq_d1 mem.
@cocotb.test()
async def read_channels_happy_path(main):
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
    await read_axi_test_helper(main, happy_data_vec, happy_data_vec)


# Adding extra data to backing mmap does not ruin reading of 16 elements and writing them correctly.
@cocotb.test()
async def read_channels_extra_mmap_data(main):
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
    await read_axi_test_helper(main, large_data_vec, large_data_vec[0:16])


# Using a small mmap will have the AXI read loop back around
# NOTE: From what I can tell, this is not part of AXI spec, but rather behavior of cocotb AXI.
# Still think it is useful to test for to see if anything breaks with this
@cocotb.test()
async def read_channels_small_mmap_data(main):
    small_data_vec = [1, 2, 4, 8, 2**32 - 1]
    expected_data_vec = [
        1,
        2,
        4,
        8,
        2**32 - 1,
        1,
        2,
        4,
        8,
        2**32 - 1,
        1,
        2,
        4,
        8,
        2**32 - 1,
        1,
    ]
    await read_axi_test_helper(main, small_data_vec, expected_data_vec)


async def read_axi_test_helper(
    module, data_vec: List[int], expected: List[int], mmap_size: int = None
):
    """Create an mmap with data of `data_vec` and use this to initialize
    a cocotb-axi-ram (read only) with this data. Assert that the data that
    our AXI program reads has been written to the memory inside our Calyx program
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
    axi_ram_read = AxiRamRead(
        # NOTE: prefix should not contain the final "_"
        AxiReadBus.from_prefix(module, "m"),
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
    assert (
        cocotb_mem_to_ints(module.vec1_data) == expected
    ), f"main.vec1_data: {cocotb_mem_to_ints(module.vec1_data)} does not contain the data in expected: {expected}."


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


# returns iterable of ints or a single int depending on size of bytes argument
def bytes_to_int(bytes, byteorder="little"):
    assert len(bytes) % 4 == 0, "bytes length not divisble by 4."
    frmt = get_format(byteorder, bytes)
    ints = struct.unpack(frmt, bytes)
    if len(ints) == 1:
        return ints[0]
    return ints


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
