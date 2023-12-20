import cocotb
from cocotb.clock import Clock
from cocotbext.axi import AxiReadBus, AxiRamRead
from cocotb.triggers import Timer, ClockCycles
import mmap
import struct
from typing import Union, Literal

# TODO(nathanielnrn): If optional signals like WSTRB are not recognized,
# install cocotb-bus directly from github, as 0.2.1 has a bug


@cocotb.test()
async def read_channels_tests(main):
    cocotb.start_soon(Clock(main.clk, 2, units="ns").start())

    # Assert reset for 5 cycles (reuqired for Calyx interfacing)
    main.reset.value = 1
    await ClockCycles(main.clk, 5)  # wait a bit
    main.reset.value = 0

    # Start the execution
    main.go.value = 1

    # anonymous mmep for now to back axiram
    memmap = mmap.mmap(-1, 128)
    axi_ram_read = AxiRamRead(
        # NOTE: prefix should not contain the final "_"
        AxiReadBus.from_prefix(main, "m"),
        main.clk,
        main.reset,
        # size in bytes
        size=128,
        mem=memmap,
    )

    vec1 = [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 4294967295]
    vec1_bytes = int_to_bytes(vec1)
    memmap.seek(0)
    memmap.write(vec1_bytes)
    memmap.seek(0)
    print(f"memmap read: {bytes_to_int(memmap.read(4*18))}")

    await Timer(20, "ns")
    # main.m_ap_start.value = 1
    axi_ram_read.hexdump(0x0000, 4 * 18, prefix="RAM")

    await Timer(1000, "ns")
    print(f"vec1_data: {main.vec1_data.mem.value}")
    assert main.vec1_data.mem[0] == b"s", "Axi channel failed"


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
