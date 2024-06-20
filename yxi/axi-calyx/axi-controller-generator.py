from calyx.builder import Builder, add_comp_ports, invoke, par, while_, if_
from typing import Literal
from math import log2
import json
import sys

# In general, ports to the wrapper are uppercase, internal registers are lower case.

# Since yxi is still young, keys and formatting change often.
width_key = "data_width"
size_key = "total_size"
name_key = "name"
# This returns an array based on dimensions of memory
address_width_key = "idx_sizes"
type_key = "memory_type"


# Adds an AXI-lite subordinate controller for XRT-managed kernels
# https://docs.amd.com/r/en-US/ug1393-vitis-application-acceleration/Control-Requirements-for-XRT-Managed-Kernels
# 0x0 to 0x0F are reserved (inclusive). Kernel arguments start at 0x10, and are 64-bits each.


# NOTE (nate): Playing around with different ways to generate these channels
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
    channel_outputs = []

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
    x_addr = s_to_m_address_channel.reg(16, f"{lc_x[1]}_addr", is_ref=True)

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

        # store addr
        x_addr.in_ = xADDR
        x_addr.write_en = (x_ready.out & xVALID) @ 1
        x_addr.write_en = ~(x_ready.out & xVALID) @ 0

        block_transfer.done = x_addr.done

    s_to_m_address_channel.control += [block_transfer]


def add_read_channel(prog):
    read_channel = create_axi_lite_channel_ports(prog, "R")

    rdata = read_channel.reg(32, "rdata", is_ref=True)
    rvalid = read_channel.reg(1, "rvalid")
    r_handshake_occurred = read_channel.reg(1, "r_handshake_ocurred")

    RREADY = read_channel.this()["RREADY"]

    with read_channel.continuous:
        read_channel.this()["RVALID"] = rvalid.out

    with read_channel.group("service_read_request") as service_read_request:
        # Complicated guard ensures RVALID is high for a single cycle, and only once per invocation
        rvalid.in_ = (~(rvalid.out & RREADY) & ~r_handshake_occurred.out) @ 1
        rvalid.in_ = ((rvalid.out & RREADY) | r_handshake_occurred.out) @ 0
        rvalid.write_en = 1

        # Goes and stays high after first handshake
        r_handshake_occurred.in_ = (rvalid.out & RREADY) @ 1
        r_handshake_occurred.in_ = ~(rvalid.out & RREADY) @ 0
        r_handshake_occurred.write_en = (~r_handshake_occurred.out) @ 1

        read_channel.this()["RDATA"] = rdata.out
        # 0b00 signals OKAY. In the future, could drive RRESP from a ref reg found in the `read_controller`
        # For faulty memory addresses could return 0b11 to signal a decode error.
        read_channel.this()["RRESP"] = 0b00

        # TODO: Make sure this works? This is changed from the manager controllers which uses a "bt_reg" (block_transfer)
        service_read_request.done = r_handshake_occurred.out

    read_channel.control += [
        invoke(r_handshake_occurred, in_in=0),
        service_read_request,
    ]


def add_write_channel(prog):
    write_channel = create_axi_lite_channel_ports(prog, "W")

    wdata = write_channel.reg(32, "wdata", is_ref=True)
    wready = write_channel.reg(1, "wready")

    with write_channel.continuous:
        write_channel.this()["WREADY"] = wready.out

    # We can get away with not having a "bt_reg/handshake_occurred" register because there will only ever be one handshake per transaction in AXI lite
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
    # In some other places this is called `bt_reg`
    b_handshake_occurred = bresp_channel.reg(1, "b_handshake_occurred")

    with bresp_channel.continuous:
        bresp_channel.this()["BVALID"] = bvalid.out
        bresp_channel.this()[
            "BRESP"
        ] = 0b00  # Assume OKAY. Could make this dynamic in the future by passing in a ref cell.

    with bresp_channel.group("block_transfer") as block_transfer:
        BREADY = bresp_channel.this()["BREADY"]
        bvalid.in_ = (~(bvalid.out & BREADY)) @ 1
        bvalid.in_ = ((bvalid.out & BREADY)) @ 0
        bvalid.write_en = 1

        b_handshake_occurred.in_ = (bvalid.out & BREADY) @ 1
        b_handshake_occurred.in_ = ~(bvalid.out & BREADY) @ 0
        b_handshake_occurred.write_en = 1
        block_transfer.done = b_handshake_occurred.out

    bresp_channel.control += [invoke(b_handshake_occurred, in_in=0), block_transfer]


def add_read_controller(prog, mems):

    add_arread_channel(prog)
    add_read_channel(prog)
    add_bresp_channel(prog)

    read_controller = prog.component("read_controller")
    read_controller_inputs = [
        ("ARESETn", 1),
        ("ARVALID", 1),
        ("ARADDR", 16),
        ("ARPROT", 3),
        ("RREADY", 1),
        ("ap_done", 1),  # signal from XRT, passed in from the entire controller
    ]

    read_controller_outputs = [
        ("ARREADY", 1),
        ("RVALID", 1),
        ("RRESP", 2),
        ("RDATA", 32),
    ]

    add_comp_ports(read_controller, read_controller_inputs, read_controller_outputs)

    # Cells
    ar_channel = read_controller.cell(
        f"ar_channel", prog.get_component(f"s_ar_channel")
    )
    r_channel = read_controller.cell(f"r_channel", prog.get_component(f"s_r_channel"))

    # Registers
    raddr = read_controller.reg(16, "raddr")

    generate_control_registers(read_controller, mems)

    # Helps construct our case control blocks below.
    # This method returns an invocation of the r_channel that
    # sends out the contents of `reg` on RDATA
    def invoke_read_channel(reg):
        return invoke(
            r_channel,
            ref_rdata=reg,
            in_ARESETn=read_controller.this()["ARESETn"],
            in_RREADY=read_controller.this()["RREADY"],
            out_RVALID=read_controller.this()["RVALID"],
            out_RRESP=read_controller.this()["RRESP"],
            out_RDATA=read_controller.this()["RDATA"],
        )

    # Addresses are specified by XRT
    # https://docs.amd.com/r/en-US/ug1393-vitis-application-acceleration/Control-Requirements-for-XRT-Managed-Kernels
    case_dict = get_xrt_case_dict(invoke_read_channel, read_controller, mems)
    addr_case = read_controller.case(raddr.out, case_dict)
    read_controller.control += [
        invoke(
            ar_channel,
            ref_r_addr = raddr,
            in_ARESETn=read_controller.this()["ARESETn"],
            in_ARVALID=read_controller.this()["ARVALID"],
            in_ARADDR=read_controller.this()["ARADDR"],
            in_ARPROT=read_controller.this()["ARPROT"],
            out_ARREADY=read_controller.this()["ARREADY"],
        ),
        addr_case,
    ]


def add_write_controller(prog, mems):
    add_awwrite_channel(prog)
    add_write_channel(prog)
    add_bresp_channel(prog)

    write_controller = prog.component("write_controller")
    write_controller_inputs = [
        ("ARESETn", 1),
        ("AWVALID", 1),
        ("AWADDR", 16),
        ("AWPROT", 3),
        ("WVALID", 1),
        ("WDATA", 32),
        ("WSTRB", 4),
        ("BREADY", 1),
    ]

    write_controller_outputs = [
        ("AWREADY", 1),
        ("WREADY", 1),
        ("BVALID", 1),
        ("BRRESP", 2),
    ]

    add_comp_ports(write_controller, write_controller_inputs, write_controller_outputs)

    # Cells
    aw_channel = write_controller.cell(
        f"aw_channel", prog.get_component(f"s_aw_channel")
    )
    w_channel = write_controller.cell(f"w_channel", prog.get_component(f"s_w_channel"))
    b_channel = write_controller.cell(f"b_channel", prog.get_component(f"s_b_channel"))

    # Registers
    w_addr = write_controller.reg(16, "w_addr")
    generate_control_registers(write_controller, mems)

    # Invocation thats writes what is on WDATA to `reg`
    def invoke_write_channel(reg):
        return invoke(
            w_channel,
            ref_wdata=reg,
            in_ARESETn=write_controller.this()["ARESETn"],
            in_WVALID=write_controller.this()["WVALID"],
            in_WDATA=write_controller.this()["WDATA"],
            in_WSTRB=write_controller.this()["WSTRB"],
            out_WREADY=write_controller.this()["WREADY"],
        )

    case_dict = get_xrt_case_dict(invoke_write_channel, write_controller, mems)
    addr_case = write_controller.case(w_addr.out, case_dict)
    write_controller.control += [
        invoke(
            aw_channel,
            ref_aw_addr=w_addr,
            in_ARESETn=write_controller.this()["ARESETn"],
            in_AWVALID=write_controller.this()["AWVALID"],
            in_AWADDR=write_controller.this()["AWADDR"],
            in_AWPROT=write_controller.this()["AWPROT"],
            out_AWREADY=write_controller.this()["AWREADY"],
        ),
        addr_case,
        invoke(
            b_channel,
            in_BREADY=write_controller.this()["BREADY"],
            out_BVALID=write_controller.this()["BVALID"],
            out_BRESP=write_controller.this()["BRRESP"],
        ),
    ]
    


def get_xrt_case_dict(invoke_function, controller, mems):
    case_dict = {
        0x0: invoke_function(controller.get_cell("control")),
        # We only need these if our kernel support interrupts
        # 0x4: invoke_function(controller.get_cell("gie")),
        # 0x8: invoke_function(controller.get_cell("iie")),
        # 0xC: invoke_function(controller.get_cell("iis")),
    }
    args_addr = 0x10
    for mem in mems:
        case_dict[args_addr] = invoke_function(
            controller.get_cell(f"{mem['name']}_base_addr")
        )
        args_addr += 8  # 64 bit addr per kernel argument is 8 bytes
    return case_dict

# Add XRT specified control registers and appropriate base_address registers for each memory
# to `component`
def generate_control_registers(component, mems):
    # XRT registers. We currently ignore everything except control and kernel argument registers
    component.reg(32, "control", is_ref=True)
    
    #We only need these if we want to support interrupts
    # # Global Interrupt Enable
    # component.reg(32, "gie", is_ref=True)
    # # IP Interrupt Enable
    # component.reg(32, "iie", is_ref=True)
    # # IP Interrupt Status
    # component.reg(32, "iis", is_ref=True)

    # These hold the base address of the memory mappings on the host
    # Kernel Arguments
    for mem in mems:
        component.reg(64, f"{mem['name']}_base_addr", is_ref=True)


# Ports must be named `s_axi_control_*` and is case sensitive.
def add_control_subordinate(prog, mems):

    add_read_controller(prog, mems)
    add_write_controller(prog, mems)
    control_subordinate = prog.component("control_subordinate")
    control_subordinate_inputs = [
        ("ARESETn", 1),
        ("AWVALID", 1),
        (
            "AWADDR",
            16,
        ),  # XRT imposes a 16-bit address space for the control subordinate
        # ("AWPROT", 3), #We don't do anything with this
        ("WVALID", 1),
        (
            "WDATA",
            32,
        ),  # Want to use 32 bits because the registers in XRT are asusemd to be this size
        (
            "WSTRB",
            32 / 8,
        )(  # We don't use this but it is required by some versions of the spec. We should tie high on subordinate.
            "BREADY", 1
        ),
        ("ARVALID", 1),
        ("ARADDR", 16),
        # ("ARPROT", 3), #We don't do anything with this
        ("RVALID", 1),
    ]

    control_subordinate_outputs = [
        ("AWREADY", 1),
        ("WREADY", 1),
        ("BVALID", 1),
        ("BRESP", 2),  # No error detection, for now we just set to 0b00 = OKAY.
        ("ARREADY", 1),
        ("RDATA", 32),
        ("RRESP", 2),  # No error detection, for now we just set to 0b00 = OKAY.
    ]

    add_comp_ports(control_subordinate, control_subordinate_inputs, control_subordinate_outputs)

    # Cells
    control = control_subordinate.reg(32, "control")
    #TODO: It could be nice to add to builder a way to access bits directly.
    # Currently: need to hook up these wires manually
    ap_start = control_subordinate.bit_slice("ap_start", 32, 0,0)
    ap_done = control_subordinate.bit_slice("ap_done", 32, 1,1)
    
    read_controller = control_subordinate.cell(
        f"read_controller", prog.get_component("read_controller")
    )
    write_controller = control_subordinate.cell(
        f"write_controller", prog.get_component("write_controller")
    )





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
    #TODO: All of these should be called heirarchically from add_control_subordinate
    check_mems_welformed(mems)
    add_read_controller(prog, mems)
    add_write_controller(prog, mems)
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
        assert (
            mem[type_key] == "Dynamic"
        ), "Only dynamic memories are currently supported for dynamic axi"


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
