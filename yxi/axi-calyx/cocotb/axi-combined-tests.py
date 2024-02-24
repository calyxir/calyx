import cocotb
from cocotb.clock import Clock
from cocotbext.axi import AxiBus, AxiRam
from cocotb.triggers import Timer, ClockCycles
import mmap
import struct
from typing import Union, Literal, List

# TODO(nathanielnrn): If optional signals like WSTRB are not recognized,
# install cocotb-bus directly from github, as 0.2.1 has a bug


debug = False


# Reads 8 elements from mmap of 8*4 bytes. Writes these elements to 8 cells in calyx defined seq_d1 mem.
@cocotb.test()
async def read_channels_happy_path(main):
    A0_in = [1, 2, 4, 8, 16, 32, 64, 128]
    B0_in = [126, 62, 30, 14, 6, 2, 0, 1]
    expected_sum = [A0_in[i] + B0_in[i] for i in range(len(B0_in))]
    await run_module(main, A0_in, B0_in, expected_sum)  # checks cocotb axi rams
    await assert_mem_content(main.internal_mem_A0, A0_in)  # checks in verilog module, as opposed to axiram
    await assert_mem_content(main.internal_mem_B0, B0_in)
    await assert_mem_content(main.internal_mem_Sum0, expected_sum)


# Adding extra data to backing mmap does not ruin reading of 8 elements and writing them correctly.
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

    await run_module(main, A0_in, B0_in, expected_sum)
    await assert_mem_content(main.internal_mem_Sum0, expected_sum[0:8])
    await assert_mem_content(main.internal_mem_A0, A0_in[0:8])
    await assert_mem_content(main.internal_mem_B0, B0_in[0:8])


##################
# Helper functions#
##################


async def assert_mem_content(mem, expected: List[int]):
    """Checks that `mem` content inside the verilog module (as opposed to
    cocotb axi-ram matches expected
    """
    if debug:
        print(f"DEBUG: assert_mem_content: {cocotb_mem_to_ints(mem)}")
    assert (
        cocotb_mem_to_ints(mem) == expected
    ), f":mem {cocotb_mem_to_ints(mem)} does not contain the data in expected: {expected}."


async def assert_axi_ram_content(
    axi_ram, expected: List[int], address=0x0000, length=8 * 4
):
    """Checks that `mem` content inside the cocotb (as opposed to
    verilog module matches expected starting at address for length bytes
    """
    if debug:
        print(f"DEBUG: axi_ram.read: {axi_ram.read(address,length)}")
        print(
            f"DEBUG: assert_axi_ram_content: {bytes_to_int(axi_ram.read(address,length))}"
        )
    assert (
        bytes_to_int(axi_ram.read(address, length)) == expected
    ), f"The axi_ram {axi_ram} contained {bytes_to_int(axi_ram.read(address,length))} not {expected}."


async def run_module(
    module,
    A0_data: List[int],
    B0_data: List[int],
    Sum0_expected: List[int],
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

    #Used to test byte-addressable to calyx-width-addressable 
    base_address = 0x1000
    # 4 bytes per integer
    A0_size = len(A0_data) * 4 + base_address
    B0_size = len(B0_data) * 4 + base_address
    Sum0_size = 8 * 4  + base_address # hardcoded because we dont pass in any sum data
    # anonymous mmep for now to back axiram
    A0_memmap = mmap.mmap(-1, A0_size)
    B0_memmap = mmap.mmap(-1, B0_size)
    Sum0_memmap = mmap.mmap(-1, Sum0_size)

    A0 = AxiRam(
        # NOTE: prefix should not contain the final "_"
        AxiBus.from_prefix(module, "A0"),
        module.clk,
        module.reset,
        # size in bytes
        size=A0_size,
        mem=A0_memmap,
    )

    B0 = AxiRam(
        # NOTE: prefix should not contain the final "_"
        AxiBus.from_prefix(module, "B0"),
        module.clk,
        module.reset,
        # size in bytes
        size=B0_size,
        mem=B0_memmap,
    )

    Sum0 = AxiRam(
        # NOTE: prefix should not contain the final "_"
        AxiBus.from_prefix(module, "Sum0"),
        module.clk,
        module.reset,
        # size in bytes
        size=Sum0_size,
        mem=Sum0_memmap,
    )

    A0_bytes = int_to_bytes(A0_data)
    B0_bytes = int_to_bytes(B0_data)

    A0_memmap.seek(base_address)
    A0_memmap.write(A0_bytes)
    A0_memmap.seek(0)

    B0_memmap.seek(base_address)
    B0_memmap.write(B0_bytes)
    B0_memmap.seek(0)

    Sum0_memmap.seek(base_address)

    await Timer(20, "ns")
    if debug:
        A0.hexdump(base_address, A0_size - base_address , prefix="A0 RAM")
        B0.hexdump(base_address, B0_size - base_address, prefix="B0 RAM")
        Sum0.hexdump(base_address, Sum0_size - base_address, prefix="Sum0 RAM")

    await Timer(1000, "ns")
    if debug:
        A0.hexdump(base_address, A0_size - base_address, prefix="A0 RAM post")
        B0.hexdump(base_address, B0_size - base_address, prefix="B0 RAM post")
        Sum0.hexdump(base_address, Sum0_size - base_address, prefix="Sum0 RAM post")

    #TODO(nathanielnrn): dynamically pass in `length` currently uses default 8*4 for vec_add test
    await assert_axi_ram_content(A0, A0_data[0:8], base_address)
    await assert_axi_ram_content(B0, B0_data[0:8], base_address)
    await assert_axi_ram_content(Sum0, Sum0_expected[0:8], base_address)

    if debug:
        print(f"A0 is: {module.internal_mem_A0.mem.value}")
        print(f"B0 is: {module.internal_mem_B0.mem.value}")
        print(f"Sum0 is: {module.internal_mem_Sum0.mem.value}")


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
