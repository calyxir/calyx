import cocotb
from cocotb.clock import Clock
from cocotbext.axi import AxiBus, AxiRam
from cocotb.triggers import Timer, ClockCycles
import mmap
import struct
from typing import Union, Literal, List

# TODO(nathanielnrn): If optional signals like WSTRB are not recognized,
# install cocotb-bus directly from github, as 0.2.1 has a bug


# Reads 8 elements from mmap of 16*4 bytes. Writes these elements to 16 cells in calyx defined seq_d1 mem.
@cocotb.test()
async def read_channels_happy_path(main):
    A0_in = [1, 2, 4, 8, 16, 32, 64, 128]
    B0_in = [127, 63, 31, 15, 7, 3, 1, 0]
    expected_sum = [A0_in[i] + B0_in[i] for i in range(len(B0_in))]
    await read_axi_test_helper(main, A0_in, B0_in, A0_in, B0_in)
    await assert_sum(main, expected_sum)


# Adding extra data to backing mmap does not ruin reading of 16 elements and writing them correctly.
@cocotb.test()
async def read_channels_extra_mmap_data(main):
    A0_in = [
        1,
        2,
        4,
        8,
        16,
        32,
        64,
        128,
        2**32 - 1,
    ]
    B0_in = [127, 63, 31, 15, 7, 3, 1, 0, 2**32 - 1]
    expected_sum = [A0_in[i] + B0_in[i] for i in range(len(B0_in))]

    await read_axi_test_helper(
        main, A0_in, B0_in, A0_in[0:8], B0_in[0:8]
    )
    await assert_sum(main, expected_sum[0:8]) 


# Using a small mmap will have the AXI read loop back around
# NOTE: From what I can tell, this is not part of AXI spec, but rather behavior of cocotb AXI.
# Still think it is useful to test for to see if anything breaks with this
# @cocotb.test()
# async def read_channels_small_mmap_data(main):
#    small_data_vec = [1, 2, 2**32 - 1]
#    expected_data_vec = [
#        1,
#        2,
#        2**32 - 1,
#        1,
#        2,
#        2**32 - 1,
#        1,
#        2,
#    ]
#    await read_axi_test_helper(main, small_data_vec, expected_data_vec)


async def assert_sum(module, expected: List[int]):
    """Checks that `Sum0` matches expected"""
    assert (
        cocotb_mem_to_ints(module.Sum0) == expected
    ), f"main.Sum0: {cocotb_mem_to_ints(module.Sum0)} does not contain the data in expected: {expected}."


async def read_axi_test_helper(
    module,
    A0_data: List[int],
    B0_data: List[int],
    A0_expected: List[int],
    B0_expected: List[int],
):
    """Create an mmap with data of `data_vec` and use this to initialize
    a cocotb-axi-ram (read only) with this data. Assert that the data that
    our AXI program reads has been written to the memory inside our Calyx program
    correctly and matches `expected.`

    mmap_size is in bytes.

    """
    cocotb.start_soon(Clock(module.clk, 2, units="ns").start())

    # Assert reset for 5 cycles (required for Calyx interfacing)
    module.reset.value = 1
    await ClockCycles(module.clk, 5)  # wait a bit
    module.reset.value = 0

    # Start the execution
    module.go.value = 1

    # 4 bytes per integer
    A0_size = len(A0_data) * 4
    B0_size = len(B0_data) * 4
    # anonymous mmep for now to back axiram
    A0_memmap = mmap.mmap(-1, A0_size)
    B0_memmap = mmap.mmap(-1, B0_size)
    A0 = AxiRam(
        # NOTE: prefix should not contain the final "_"
        AxiBus.from_prefix(module, "m0"),
        module.clk,
        module.reset,
        # size in bytes
        size=A0_size,
        mem=A0_memmap,
    )

    B0 = AxiRam(
        # NOTE: prefix should not contain the final "_"
        AxiBus.from_prefix(module, "m1"),
        module.clk,
        module.reset,
        # size in bytes
        size=B0_size,
        mem=B0_memmap,
    )

    A0_bytes = int_to_bytes(A0_data)
    B0_bytes = int_to_bytes(B0_data)

    A0_memmap.seek(0)
    A0_memmap.write(A0_bytes)
    A0_memmap.seek(0)

    B0_memmap.seek(0)
    B0_memmap.write(B0_bytes)
    B0_memmap.seek(0)

    await Timer(20, "ns")
    A0.hexdump(0x0000, A0_size, prefix="A0 RAM")
    B0.hexdump(0x0000, B0_size, prefix="B0 RAM")

    debug = True

    await Timer(500, "ns")

    if debug:
        print(f"A0 is: {module.A0.mem.value}")
        print(f"B0 is: {module.B0.mem.value}")
        print(f"Sum0 is: {module.Sum0.mem.value}")

    assert (
        cocotb_mem_to_ints(module.A0) == A0_expected
    ), f"main.A0: {cocotb_mem_to_ints(module.A0)} does not contain the data in expected: {A0_expected}."
    assert (
        cocotb_mem_to_ints(module.B0) == B0_expected
    ), f"main.B0: {cocotb_mem_to_ints(module.B0)} does not contain the data in expected: {B0_expected}."


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
