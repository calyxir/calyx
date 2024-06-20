from calyx.builder import (
    Builder,
    add_comp_ports,
    invoke,
    par,
    while_,
    if_
)
from typing import Literal
from math import log2
import json
import sys

# In general, ports to the wrapper are uppercase, internal registers are lower case.

# Since yxi is still young, keys and formatting change often.
width_key = "data_width"
size_key = "total_size"
name_key = "name"
#This returns an array based on dimensions of memory
address_width_key = "idx_sizes"
type_key = "memory_type"


# Adds an AXI-lite subordinate controller for XRT-managed kernels
# https://docs.amd.com/r/en-US/ug1393-vitis-application-acceleration/Control-Requirements-for-XRT-Managed-Kernels
# 0x0 to 0x0F are reserved (inclusive). Kernel arguments start at 0x10, and are 64-bits each.


#NOTE (nate): Playing around with different ways to generate these channels
# In general there is some shared ports/logic, but also enough to warrant separate
# functions. Haven't yet landed on something that feels "best". The dynamic and static
# memory axi controllers channel generation is largely isolated for each channel.
# This merges port creation, but not sure this is "worth it".

def create_axi_lite_channel_ports(prog, prefix: Literal["AW", "AR", "W", "B", "R"]):
    """Adds an AXI-lite subordinate-to-manager address channel.

    Returns a component builder in case there are additional
    cells/wires/groups that need to be added to the component.
    """

    # Following Arm's notation of denoting `xVALID` and `xREADY` signals
    # `x` stands for the prefix of the channel, i.e. `AW` or `AR`
    lc_x = prefix.lower()
    x = prefix
    s_to_m_channel = prog.component(f"s_{lc_x}_channel")
    channel_inputs = [
        ("ARESETn", 1),
    ]
    channel_outputs = [
    ]

    if x in ["AW", "AR"]:
        channel_inputs.append((f"{x}VALID", 1))
        channel_inputs.append((f"{x}ADDR", 16))
        channel_inputs.append((f"{x}PROT", 3))
        channel_outputs.append((f"{x}READY", 1))
    elif x == "W":
        channel_inputs.append((f"WVALID", 1))
        channel_inputs.append((f"WDATA", 32))
        channel_inputs.append((f"WSTRB", 2))
        channel_outputs.append((f"WREADY", 1))
    elif x in ["B", "R"]:
        channel_inputs.append((f"{x}READY", 1))
        channel_outputs.append((f"{x}VALID", 1))
        channel_outputs.append((f"{x}RESP", 2))
        if x == "R":
            channel_outputs.append((f"RDATA", 32))
    
    add_comp_ports(s_to_m_channel, channel_inputs, channel_outputs)

    return s_to_m_channel



def add_arread_channel(prog):
    _add_s_to_m_address_channel(prog, "AR")

def add_awwrite_channel(prog):
    _add_s_to_m_address_channel(prog, "AW")

def _add_s_to_m_address_channel(prog, prefix: Literal["AW", "AR"]):

    assert prefix in ["AW", "AR"], "Prefix must be either AW or AR."


    # Following Arm's notation of denoting `xVALID` and `xREADY` signals
    # `x` stands for the prefix of the channel, i.e. `AW` or `AR`
    lc_x = prefix.lower()
    x = prefix
    s_to_m_address_channel = create_axi_lite_channel_ports(prog, x)
    x_ready = s_to_m_address_channel.reg(1, f"{lc_x}_ready")
    x_addr = s_to_m_address_channel.reg(16, f"{lc_x[1]}_addr", is_ref = True)

    with s_to_m_address_channel.continuous:
        s_to_m_address_channel.this()[f"{x}READY"] = x_ready.out
        x_addr.in_ = s_to_m_address_channel.this()[f"{x}ADDR"]

    with s_to_m_address_channel.group("block_transfer") as block_transfer:
        xVALID = s_to_m_address_channel.this()[f"{x}VALID"]
        xADDR = s_to_m_address_channel.this()[f"{x}ADDR"]

        # ar_ready.in = 1 does not work because it leaves ARREADY high for 2 cycles.
        # The way it is below leaves it high for only 1 cycle.  See #1828
        # https://github.com/calyxir/calyx/issues/1828
        x_ready.in_ = ~(x_ready.out & xVALID) @ 1
        x_ready.in_ = (x_ready.out & xVALID) @ 0
        x_ready.write_en = 1

        #store addr
        x_addr.in_ = xADDR
        x_addr.write_en = (x_ready.out & xVALID) @ 1
        x_addr.write_en = ~(x_ready.out & xVALID) @ 0
        
        block_transfer.done = x_addr.done

    s_to_m_address_channel.control += [block_transfer]
    
def add_read_channel(prog):
    read_channel = create_axi_lite_channel_ports(prog, "R")
    
    rdata = read_channel.reg(32, "rdata", is_ref = True)
    rvalid = read_channel.reg(1, "rvalid")
    r_handshake_occurred = read_channel.reg(1, "r_handshake_ocurred")

    RREADY = read_channel.this()["RREADY"]

    with read_channel.continuous:
        read_channel.this()["RVALID"] = rvalid.out
    
    with read_channel.group("service_read_request") as service_read_request:
        
        #Complicated guard ensures RVALID is high for a single cycle, and only once per invocation
        rvalid.in_ = (~(rvalid.out & RREADY) & ~r_handshake_occurred.out) @ 1
        rvalid.in_ = ((rvalid.out & RREADY) | r_handshake_occurred.out) @ 0
        rvalid.write_en = 1

        #Goes and stays high after first handshake
        r_handshake_occurred.in_ = (rvalid.out & RREADY) @ 1
        r_handshake_occurred.in_ = ~(rvalid.out & RREADY) @ 0
        r_handshake_occurred.write_en = (~r_handshake_occurred.out) @ 1

        read_channel.this()["RDATA"] = rdata.out
        #0b00 signals OKAY. In the future, could drive RRESP from a ref reg found in the `read_controller`
        # For faulty memory addresses could return 0b11 to signal a decode error.
        read_channel.this()["RRESP"] = 0b00

        #TODO: Make sure this works? This is changed from the manager controllers which uses a "bt_reg" (block_transfer)
        service_read_request.done = r_handshake_occurred.out

    read_channel.control += [
        invoke(r_handshake_occurred, in_in = 0),
        service_read_request,
    ]

def add_write_channel(prog):
    write_channel = create_axi_lite_channel_ports(prog, "W")

    wdata = write_channel.reg(32, "wdata", is_ref = True)
    wready = write_channel.reg(1, "wready")

    with write_channel.continuous:
        write_channel.this()["WREADY"] = wready.out

    #We can get away with not having a "bt_reg/handshake_occurred" register because there will only ever be one handshake per transaction in AXI lite
    with write_channel.group("service_write_request") as service_write_request:
        wVALID = write_channel.this()["WVALID"]
        wDATA = write_channel.this()["WDATA"]

        wready.in_ = (~(wready.out & wVALID)) @ 1
        wready.in_ = ((wready.out & wVALID)) @ 0
        wready.write_en = 1

        wdata.in_ = wDATA
        wdata.write_en = (wready.out & wVALID) @ 1
        wdata.write_en = ~(wready.out & wVALID) @ 0

        service_write_request.done = wdata.done

    write_channel.control += [service_write_request]


def add_bresp_channel(prog):
    bresp_channel = create_axi_lite_channel_ports(prog, "B")
    
    bvalid = bresp_channel.reg(1, "bvalid")
    #In some other places this is called `bt_reg`
    b_handshake_occurred = bresp_channel.reg(1, "b_handshake_occurred")

    with bresp_channel.continuous:
        bresp_channel.this()["BVALID"] = bvalid.out
        bresp_channel.this()["BRESP"] = 0b00 # Assume OKAY. Could make this dynamic in the future by passing in a ref cell.

    with bresp_channel.group("block_transfer") as block_transfer:
        BREADY = bresp_channel.this()["BREADY"]
        bvalid.in_ = (~(bvalid.out & BREADY)) @ 1
        bvalid.in_ = ((bvalid.out & BREADY)) @ 0
        bvalid.write_en = 1


        b_handshake_occurred.in_ = (bvalid.out & BREADY) @ 1
        b_handshake_occurred.in_ = ~(bvalid.out & BREADY) @ 0
        b_handshake_occurred.write_en = 1
        block_transfer.done = b_handshake_occurred.out


    bresp_channel.control += [invoke(b_handshake_occurred, in_in = 0), block_transfer]


def add_read_controller(prog, mems):
    read_controller = prog.component("read_controller")
    read_controller_inputs = [
        ("ARESETn", 1),
        ("ARVALID", 1),
        ("ARADDR", 16),
        ("ARPROT", 3),
        ("RREADY", 1)
        ("ap_done", 1) #signal from XRT, passed in from the entire controller
    ]

    read_controller_outputs = [
        ("ARREADY", 1),
        ("RVALID", 1),
        ("RRESP", 2),
        ("RDATA", 32),
    ]

    read_controller.add_ports(read_controller_inputs, read_controller_outputs)

    #Cells
    ar_channel = read_controller.cell(f"ar_channel", prog.get_component(f"s_ar_channel"))
    r_channel = read_controller.cell(f"r_channel", prog.get_component(f"s_r_channel"))

    #XRT registers. We currently ignore everything except control and kernel argument registers

    control = read_controller.reg(32, "control_reg", is_ref = True)
    #Global Interrupt Enable
    gie = read_controller.reg(32, "gie", is_ref = True)
    #IP Interrupt Enable
    iie = read_controller.reg(32, "iie", is_ref = True)
    #IP Interrupt Status
    iis = read_controller.reg(32, "iis", is_ref = True)
    #These hold the base address of the memory mappings on the host
    #Kernel Arguments
    for mem in mems:
        read_controller.reg(64, f"{mem['name']}_base_addr", is_ref = True)

    read_controller.control +



#Ports must be named `s_axi_control_*` and is case sensitive.
def add_control_subordinate(prog, mems):
    control_subordinate = prog.component("control_subordinate")
    control_subordinate_inputs = [
        ("ARESETn", 1),
        ("AWVALID", 1),
        ("AWADDR", 16) #XRT imposes a 16-bit address space for the control subordinate
        # ("AWPROT", 3), #We don't do anything with this
        ("WVALID", 1),
        ("WDATA", 32) #Want to use 32 bits because the registers in XRT are asusemd to be this size
        ("WSTRB", 32/8), #We don't use this but it is required by some versions of the spec. We should tie high on subordinate.
        ("BREADY", 1),

        ("ARVALID", 1),
        ("ARADDR", 16),
        # ("ARPROT", 3), #We don't do anything with this
        ("RVALID", 1),


        
    ]

    control_subordinate_outputs = [
        ("AWREADY", 1),
        ("WREADY", 1)
        ("BVALID", 1)
        ("BRESP", 2), #No error detection, for now we just set to 0b00 = OKAY.

        ("ARREADY", 1),
        ("RDATA", 32),
        ("RRESP", 2) #No error detection, for now we just set to 0b00 = OKAY.
    ]




#########################################
#########################################
#########################################
#########################################


# Helper functions
def width_in_bytes(width: int):
    assert width % 8 == 0, "Width must be a multiple of 8."
    return width // 8


def width_xsize(width: int):
    log = log2(width_in_bytes(width))
    assert log.is_integer(), "Width must be a power of 2."
    return int(log)


def clog2(x):
    """Ceiling log2"""
    if x <= 0:
        raise ValueError("x must be positive")
    return (x - 1).bit_length()


def clog2_or_1(x):
    """Ceiling log2 or 1 if clog2(x) == 0"""
    return max(1, clog2(x))


def build():
    prog = Builder()
    check_mems_welformed(mems)
    add_arread_channel(prog)
    add_awwrite_channel(prog)
    add_read_channel(prog)
    add_write_channel(prog)
    add_bresp_channel(prog)
    return prog.program


def check_mems_welformed(mems):
    """Checks if memories from yxi are well formed. Returns true if they are, false otherwise."""
    for mem in mems:
        assert (
            mem[width_key] % 8 == 0
        ), "Width must be a multiple of 8 to alow byte addressing to host"
        assert log2(
            mem[width_key]
        ).is_integer(), "Width must be a power of 2 to be correctly described by xSIZE"
        assert mem[size_key] > 0, "Memory size must be greater than 0"
        assert mem[type_key] == "Dynamic", "Only dynamic memories are currently supported for dynamic axi"


if __name__ == "__main__":
    yxi_filename = "input.yxi"  # default
    if len(sys.argv) > 2:
        raise Exception("The controller generator takes 1 yxi file name as argument")
    else:
        try:
            yxi_filename = sys.argv[1]
            if not yxi_filename.endswith(".yxi"):
                raise Exception("controller generator requires an yxi file")
        except:
            pass  # no arg passed
    with open(yxi_filename, "r", encoding="utf-8") as yxifile:
        yxifile = open(yxi_filename)
        yxi = json.load(yxifile)
        mems = yxi["memories"]
        build().emit()