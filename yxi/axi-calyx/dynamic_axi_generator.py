from calyx.builder import (
    Builder,
    add_comp_ports,
    invoke,
    par,
    while_,
    if_
)
from axi_controller_generator import add_control_subordinate
from typing import Literal
from math import log2, ceil
import json
import sys


GENERATE_FOR_XILINX = True

# In general, ports to the wrapper are uppercase, internal registers are lower case.

# Since yxi is still young, keys and formatting change often.
width_key = "data_width"
size_key = "total_size"
name_key = "name"
#This returns an array based on dimensions of memory
address_width_key = "idx_sizes"
type_key = "memory_type"

#TODO (nathanielnrn): Should we make these comb groups? 
def add_address_translator(prog, mem):
    address_width = mem[address_width_key][0]
    data_width = mem[width_key]
    name = mem[name_key]
    # Inputs/Outputs
    address_translator = prog.comb_component(f"address_translator_{name}")
    translator_inputs = [("calyx_mem_addr", address_width)]
    translator_output = [("axi_address", 64)]
    add_comp_ports(address_translator, translator_inputs, translator_output)

    #Cells
    #XRT expects 64 bit address.
    address_mult = address_translator.const_mult(64, width_in_bytes(data_width), f"mul_{name}")
    pad_input_addr = address_translator.pad(address_width, 64, f"pad_input_addr")


    #Assignment
    with address_translator.continuous:
        pad_input_addr.in_ = address_translator.this()["calyx_mem_addr"]
        address_mult.in_ = pad_input_addr.out
        address_translator.this()["axi_address"] = address_mult.out


def add_arread_channel(prog, mem):
    _add_m_to_s_address_channel(prog, mem, "AR")


def add_awwrite_channel(prog, mem):
    _add_m_to_s_address_channel(prog, mem, "AW")


def _add_m_to_s_address_channel(prog, mem, prefix: Literal["AW", "AR"]):
    """Adds a manager to subordinate
    channel to the program. Uses `prefix` to name the channels
    appropriately. Expected to be either "AW" or "AR."
    Contains all of the channels shared between AW and AR channels.

    Returns a component builder in case there are additional
    cells/wires/groups that need to be added to the component.
    """

    assert prefix in ["AW", "AR"], "Prefix must be either AW or AR."

    # Following Arm's notation of denoting `xVALID` and `xREADY` signals
    # `x` stands for the prefix of the channel, i.e. `AW` or `AR`
    lc_x = prefix.lower()
    x = prefix
    name = mem[name_key]
    # Inputs/outputs
    m_to_s_address_channel = prog.component(f"m_{lc_x}_channel_{name}")
    channel_inputs = [("ARESETn", 1), (f"{x}READY", 1), ("axi_address", 64)]
    channel_outputs = [
        (f"{x}VALID", 1),
        (f"{x}ADDR", 64),
        (f"{x}SIZE", 3),  # bytes used in transfer
        (f"{x}LEN", 8),  # number of transfers in transaction
        (f"{x}BURST", 2),  # for XRT should be tied to 2'b01 for WRAP burst
        (f"{x}PROT", 3),  # tied to be priviliged, nonsecure, data access request
    ]
    add_comp_ports(m_to_s_address_channel, channel_inputs, channel_outputs)

    # Cells
    xvalid = m_to_s_address_channel.reg(1, f"{lc_x}valid")
    xhandshake_occurred = m_to_s_address_channel.reg(1, f"{lc_x}_handshake_occurred")

    # Need to put block_transfer register here to avoid combinational loops
    bt_reg = m_to_s_address_channel.reg(1, "bt_reg")

    # Wires
    with m_to_s_address_channel.continuous:
        m_to_s_address_channel.this()[f"{x}VALID"] = xvalid.out

    # Groups
    # Responsible for asserting ARVALID, and deasserting it a cycle after the handshake.
    # This is necesarry because of the way transitions between groups work.
    # See #1828 https://github.com/calyxir/calyx/issues/1828
    with m_to_s_address_channel.group(f"do_{lc_x}_transfer") as do_x_transfer:
        xREADY = m_to_s_address_channel.this()[f"{x}READY"]
        xvalid.in_ = (~(xvalid.out & xREADY) & ~xhandshake_occurred.out) @ 1
        # Deassert in the next cycle once it is high
        xvalid.in_ = ((xvalid.out & xREADY) | xhandshake_occurred.out) @ 0
        xvalid.write_en = 1

        xhandshake_occurred.in_ = (xvalid.out & xREADY) @ 1
        xhandshake_occurred.write_en = (~xhandshake_occurred.out) @ 1

        # Drive output signals for transfer
        m_to_s_address_channel.this()[f"{x}ADDR"] = m_to_s_address_channel.this()["axi_address"]
        # This is taken from mem size, we assume the databus width is the size
        # of our memory cell and that width is a power of 2
        # TODO(nathanielnrn): convert to binary instead of decimal
        m_to_s_address_channel.this()[f"{x}SIZE"] = width_xsize(mem[width_key])
        #Dynamic accesses only need  asingle transfer per transcation
        m_to_s_address_channel.this()[f"{x}LEN"] = 0
        m_to_s_address_channel.this()[f"{x}BURST"] = 1  # Must be INCR for XRT
        # Required by spec, we hardcode to privileged, non-secure, data access
        m_to_s_address_channel.this()[f"{x}PROT"] = 0b110

        # control block_transfer reg to go low after one cycle
        bt_reg.in_ = (xREADY & xvalid.out) @ 1
        bt_reg.in_ = ~(xREADY & xvalid.out) @ 0
        bt_reg.write_en = 1
        do_x_transfer.done = bt_reg.out


    # ARLEN must be between 0-255, make sure to subtract 1 from yxi
    # size when assigning to ARLEN
    # assert mem[size_key] < 256, "Memory size must be less than 256"


    m_to_s_address_channel.control += [
        par(
            invoke(bt_reg, in_in=0),
            invoke(xhandshake_occurred, in_in=0),
        ),
        do_x_transfer,
        invoke(xvalid, in_in=0),
    ]
    return m_to_s_address_channel


def add_read_channel(prog, mem):
    # Inputs/Outputs
    name = mem[name_key]
    read_channel = prog.component(f"m_read_channel_{name}")
    # TODO(nathanielnrn): We currently assume RDATA is the same width as the
    # memory. This limits throughput many AXI data busses are much wider
    # i.e., 512 bits.
    channel_inputs = [
        ("ARESETn", 1),
        ("RVALID", 1),
        ("RLAST", 1),
        ("RDATA", mem[width_key]),
        ("RRESP", 2),
    ]
    channel_outputs = [("RREADY", 1), ("read_data", mem[width_key])]
    add_comp_ports(read_channel, channel_inputs, channel_outputs)

    # Cells

    # We assume idx_size is exactly clog2(len). See comment in #1751
    # https://github.com/calyxir/calyx/issues/1751#issuecomment-1778360566
    read_reg = read_channel.reg(mem[width_key], "read_reg")

    # according to zipcpu, rready should be registered
    rready = read_channel.reg(1, "rready")
    # Registed because RLAST is high with laster transfer, not after
    # before this we were terminating immediately with
    # last transfer and not servicing it
    n_RLAST = read_channel.reg(1, "n_RLAST")
    # Stores data we want to write to our memory at end of block_transfer group
    # read_data_reg = read_channel.reg("read_data_reg", mem[width_key])

    # bt_reg = read_channel.reg("bt_reg", 1)

    # Groups
    with read_channel.continuous:
        read_channel.this()["RREADY"] = rready.out
        read_channel.this()["read_data"] = read_reg.out

    # Wait for handshake. Ensure that when this is done we are ready to write
    # (i.e., read_data_reg.write_en = is_rdy.out)
    # xVALID signals must be high until xREADY is high too, this works because
    # if xREADY is high, then xVALID being high makes 1 flip and group
    # is done by bt_reg.out
    with read_channel.group("block_transfer") as block_transfer:
        RVALID = read_channel.this()["RVALID"]
        RDATA = read_channel.this()["RDATA"]
        RLAST = read_channel.this()["RLAST"]
        # TODO(nathanielnrn): We are allowed to have RREADY depend on RVALID.
        # Can we simplify to just RVALID?

        # rready.in = 1 does not work because it leaves RREADY high for 2 cycles.
        # The way it is below leaves it high for only 1 cycle.  See #1828
        # https://github.com/calyxir/calyx/issues/1828

        # TODO(nathanielnrn): Spec recommends defaulting xREADY high to get rid
        # of extra cycles.  Can we do this as opposed to waiting for RVALID?
        rready.in_ = ~(rready.out & RVALID) @ 1
        rready.in_ = (rready.out & RVALID) @ 0
        rready.write_en = 1

        # Store data we want to write
        read_reg.in_ = RDATA
        read_reg.write_en = (rready.out & RVALID) @ 1
        read_reg.write_en = ~(rready.out & RVALID) @ 0

        n_RLAST.in_ = RLAST @ 0
        n_RLAST.in_ = ~RLAST @ 1
        n_RLAST.write_en = 1

        # We are done after handshake
        # bt_reg.in_ = (rready.out & RVALID) @ 1
        # bt_reg.in_ = ~(rready.out & RVALID) @ 0
        # bt_reg.write_en = 1
        block_transfer.done = read_reg.done

    # creates group that increments curr_addr_internal_mem by 1. Creates adder and wires up correctly
    # Control
    invoke_n_RLAST = invoke(n_RLAST, in_in=1)
    # invoke_bt_reg = invoke(bt_reg, in_in=0)
    
    # Could arguably get rid of this while loop for the dynamic verison, but this
    # matches nicely with non dynamic version and conforms to spec,
    # and will be easier to extend to variable length dynamic transfers in the future
    while_body = [
        # invoke_bt_reg,
        block_transfer,
    ]
    while_n_RLAST = while_(n_RLAST.out, while_body)

    read_channel.control += [invoke_n_RLAST, while_n_RLAST]


def add_write_channel(prog, mem):
    data_width = mem[width_key]
    name = mem[name_key]
    # Inputs/Outputs
    write_channel = prog.component(f"m_write_channel_{name}")
    channel_inputs = [
        ("ARESETn", 1),
        ("WREADY", 1),
        ("write_data", data_width)
    ]
    # TODO(nathanielnrn): We currently assume WDATA is the same width as the
    # memory. This limits throughput many AXI data busses are much wider
    # i.e., 512 bits.
    channel_outputs = [
        ("WVALID", 1),
        ("WLAST", 1),
        ("WDATA", mem[width_key]),
    ]
    add_comp_ports(write_channel, channel_inputs, channel_outputs)

    # Cells
    # We assume idx_size is exactly clog2(len). See comment in #1751

    # according to zipcpu, rready should be registered
    wvalid = write_channel.reg(1, "wvalid")
    w_handshake_occurred = write_channel.reg(1, "w_handshake_occurred")

    # Register because w last is high with last transfer. Before this
    # We were terminating immediately with last transfer and not servicing it.

    bt_reg = write_channel.reg(1, "bt_reg")

    # Groups
    with write_channel.continuous:
        write_channel.this()["WVALID"] = wvalid.out

    with write_channel.group("service_write_transfer") as service_write_transfer:
        WREADY = write_channel.this()["WREADY"]

        # Assert then deassert. Can maybe get rid of w_handshake_occurred in guard
        wvalid.in_ = (~(wvalid.out & WREADY) & ~w_handshake_occurred.out) @ 1
        wvalid.in_ = ((wvalid.out & WREADY) | w_handshake_occurred.out) @ 0
        wvalid.write_en = 1

        # Set high when wvalid is high even once
        # This is just wavlid.in_ guard from above
        w_handshake_occurred.in_ = (wvalid.out & WREADY) @ 1
        w_handshake_occurred.in_ = ~(wvalid.out & WREADY) @ 0
        w_handshake_occurred.write_en = (~w_handshake_occurred.out) @ 1

        write_channel.this()["WDATA"] = write_channel.this()["write_data"]
        write_channel.this()["WLAST"] = 1

        # done after handshake
        #TODO(nathanielnrn): Perhaps we can combine between handshake_occurred and bt_reg
        bt_reg.in_ = (wvalid.out & WREADY) @ 1
        bt_reg.in_ = ~(wvalid.out & WREADY) @ 0
        bt_reg.write_en = 1
        service_write_transfer.done = bt_reg.out

    # TODO(nathanielnrn): Currently we assume that width is a power of 2.
    # In the future we should allow for non-power of 2 widths, will need some
    # splicing for this.
    # See https://cucapra.slack.com/archives/C05TRBNKY93/p1705587169286609?thread_ts=1705524171.974079&cid=C05TRBNKY93 # noqa: E501

    # Control
    write_channel.control += [
        invoke(bt_reg, in_in=0),
        invoke(w_handshake_occurred, in_in=0),
        service_write_transfer,
    ]


# For now we assume all responses are OKAY because we don't have any error
# handling logic. So basically this sets BREADY high then lowers it on
# handshake.
def add_bresp_channel(prog, mem):
    name = mem[name_key]
    # Inputs/Outputs
    bresp_channel = prog.component(f"m_bresp_channel_{name}")
    # No BRESP because it is ignored, i.e we assume it is tied OKAY
    channel_inputs = [("ARESETn", 1), ("BVALID", 1)]
    channel_outputs = [("BREADY", 1)]
    add_comp_ports(bresp_channel, channel_inputs, channel_outputs)

    # Cells
    bready = bresp_channel.reg(1, "bready")
    bt_reg = bresp_channel.reg(1, "bt_reg")

    # Groups
    with bresp_channel.continuous:
        bresp_channel.this()["BREADY"] = bready.out

    # TODO(nathanielnrn): This is probably unoptimal and takes multiple
    # cycles to do a simple handshake which we basically ignore. Can
    # probably be much better.
    with bresp_channel.group("block_transfer") as block_transfer:
        BVALID = bresp_channel.this()["BVALID"]
        bready.in_ = ~(bready.out & BVALID) @ 1
        bready.in_ = (bready.out & BVALID) @ 0
        bready.write_en = 1

        bt_reg.in_ = (bready.out & BVALID) @ 1
        bt_reg.in_ = ~(bready.out & BVALID) @ 0
        bt_reg.write_en = 1
        block_transfer.done = bt_reg.out

    # Control
    bresp_channel.control += [invoke(bt_reg, in_in=0), block_transfer]

def add_read_controller(prog, mem):
    add_arread_channel(prog, mem)
    add_read_channel(prog, mem)

    data_width = mem[width_key]
    name = mem[name_key]

    read_controller = prog.component(f"read_controller_{name}")
    # Inputs/Outputs
    read_controller_inputs = [
        ("axi_address", 64),
        (f"ARESETn", 1),
        (f"ARREADY", 1),
        (f"RVALID", 1),
        (f"RLAST", 1),
        (f"RDATA", data_width),
        (f"RRESP", 2),
    ]

    read_controller_outputs = [
        (f"ARVALID", 1),
        (f"ARADDR", 64),
        (f"ARSIZE", 3),
        (f"ARLEN", 8),
        (f"ARBURST", 2),
        (f"ARPROT", 3),
        (f"RREADY", 1),
        #sent out to axi_dyn_mem
        (f"read_data", data_width),
    ]

    add_comp_ports(read_controller, read_controller_inputs, read_controller_outputs)

    #Cells
    simple_ar_channel = read_controller.cell(f"ar_channel_{name}", prog.get_component(f"m_ar_channel_{name}"))
    simple_read_channel = read_controller.cell(f"read_channel_{name}", prog.get_component(f"m_read_channel_{name}"))
    # No groups necesarry


    # Control
    #   Invokes

    with read_controller.continuous:
        read_controller.this()["read_data"] = simple_read_channel.read_data

    simple_ar_invoke = invoke(
        simple_ar_channel,
        in_axi_address=read_controller.this()["axi_address"],
        in_ARESETn=read_controller.this()["ARESETn"],
        in_ARREADY=read_controller.this()["ARREADY"],
        out_ARVALID=read_controller.this()["ARVALID"],
        out_ARADDR=read_controller.this()["ARADDR"],
        out_ARSIZE=read_controller.this()["ARSIZE"],
        out_ARLEN=read_controller.this()["ARLEN"],
        out_ARBURST=read_controller.this()["ARBURST"],
        out_ARPROT=read_controller.this()["ARPROT"],
    )
    simple_read_invoke = invoke(
        simple_read_channel,
        in_ARESETn=read_controller.this()["ARESETn"],
        in_RVALID=read_controller.this()["RVALID"],
        in_RLAST=read_controller.this()["RLAST"],
        in_RDATA=read_controller.this()["RDATA"],
        in_RRESP=read_controller.this()["RRESP"],
        out_RREADY=read_controller.this()["RREADY"],
        # out_read_data=read_controller.this()["read_data"],
    )
    read_controller.control += [
        simple_ar_invoke,
        simple_read_invoke,
    ]

def add_write_controller(prog, mem):
    add_awwrite_channel(prog, mem)
    add_write_channel(prog, mem)
    add_bresp_channel(prog, mem)
    data_width = mem[width_key]
    name = mem[name_key]

    write_controller = prog.component(f"write_controller_{name}")
    # Inputs/Outputs
    write_controller_inputs = [
        ("axi_address", 64),
        ("write_data", data_width),
        (f"ARESETn", 1),
        (f"AWREADY", 1),
        (f"WREADY", 1),
        (f"BVALID", 1),
    ]

    write_controller_outputs = [
        (f"AWVALID", 1),
        (f"AWADDR", 64),
        (f"AWSIZE", 3),
        (f"AWLEN", 8),
        (f"AWBURST", 2),
        (f"AWPROT", 3),
        (f"WVALID", 1),
        (f"WLAST", 1),
        (f"WDATA", data_width),
        (f"BREADY", 1),
    ]

    add_comp_ports(write_controller, write_controller_inputs, write_controller_outputs)

    #Cells
    simple_aw_channel = write_controller.cell(f"aw_channel_{name}", prog.get_component(f"m_aw_channel_{name}"))
    simple_write_channel = write_controller.cell(f"write_channel_{name}", prog.get_component(f"m_write_channel_{name}"))
    simple_bresp_channel = write_controller.cell(f"bresp_channel_{name}", prog.get_component(f"m_bresp_channel_{name}"))
    # No groups necesarry


    # Control
    #   Invokes
    simple_aw_invoke = invoke(
        simple_aw_channel,
        in_axi_address=write_controller.this()["axi_address"],
        in_ARESETn=write_controller.this()["ARESETn"],
        in_AWREADY=write_controller.this()["AWREADY"],
        out_AWVALID=write_controller.this()["AWVALID"],
        out_AWADDR=write_controller.this()["AWADDR"],
        out_AWSIZE=write_controller.this()["AWSIZE"],
        out_AWLEN=write_controller.this()["AWLEN"],
        out_AWBURST=write_controller.this()["AWBURST"],
        out_AWPROT=write_controller.this()["AWPROT"],
    )
    simple_write_invoke = invoke(
        simple_write_channel,
        in_write_data=write_controller.this()["write_data"],
        in_ARESETn=write_controller.this()["ARESETn"],
        in_WREADY=write_controller.this()["WREADY"],
        out_WVALID=write_controller.this()["WVALID"],
        out_WLAST=write_controller.this()["WLAST"],
        out_WDATA=write_controller.this()["WDATA"],
    )

    simple_bresp_invoke = invoke(
        simple_bresp_channel,
        in_BVALID=write_controller.this()["BVALID"],
        out_BREADY=write_controller.this()["BREADY"],
    )

    write_controller.control += [
        simple_aw_invoke,
        simple_write_invoke,
        simple_bresp_invoke,
    ]

def add_axi_dyn_mem(prog, mem):
    address_width = mem[address_width_key][0]
    data_width = mem[width_key]
    name = mem[name_key]

    prog.import_("primitives/memories/dyn.futil")
    axi_dyn_mem = prog.component(f"axi_dyn_mem_{name}")
    # Inputs/Outputs
    dyn_mem_inputs =[
        ("addr0", address_width, [("write_together", 1), "data"]),
        ("content_en", 1, [("write_together", 1), ("go", 1)]),
        ("write_en", 1, [("write_together", 2)]),
        ("write_data", data_width, [("write_together", 2), "data"]),
        (f"base_address", 64),
        (f"ARESETn", 1),
        (f"ARREADY", 1),
        (f"RVALID", 1),
        (f"RLAST", 1),
        (f"RDATA", mem[width_key]),
        (f"RRESP", 2),
        (f"AWREADY", 1),
        (f"WREADY", 1),
        (f"BVALID", 1),
        # Only used for waveform tracing, not sent anywhere
        (f"BRESP", 2),
    ]
    dyn_mem_outputs = [
        ("read_data", data_width, ["stable"]),
        (f"ARVALID", 1),
        (f"ARADDR", 64),
        (f"ARSIZE", 3),
        (f"ARLEN", 8),
        (f"ARBURST", 2),
        (f"ARPROT", 3),
        (f"RREADY", 1),
        (f"AWVALID", 1),
        (f"AWADDR", 64),
        (f"AWSIZE", 3),
        (f"AWLEN", 8),
        (f"AWBURST", 2),
        (f"AWPROT", 3),
        (f"WVALID", 1),
        (f"WLAST", 1),
        (f"WDATA", mem[width_key]),
        (f"BREADY", 1),
    ]
    add_comp_ports(axi_dyn_mem, dyn_mem_inputs, dyn_mem_outputs)

    # Cells
    address_translator = axi_dyn_mem.cell(f"address_translator_{name}", prog.get_component(f"address_translator_{name}"))
    read_controller = axi_dyn_mem.cell(f"read_controller_{name}", prog.get_component(f"read_controller_{name}"))
    write_controller = axi_dyn_mem.cell(f"write_controller_{name}", prog.get_component(f"write_controller_{name}"))
    base_addr_adder = axi_dyn_mem.add(64, f"base_addr_adder_{name}")
    write_en_reg = axi_dyn_mem.reg(1, f"write_en_reg_{name}")

    # Wires
    this_component = axi_dyn_mem.this()
    #  Continuous assignment
    with axi_dyn_mem.continuous:
        address_translator.calyx_mem_addr = this_component["addr0"]
        axi_dyn_mem.this()["read_data"] = read_controller.read_data
        base_addr_adder.left = this_component["base_address"]
        base_addr_adder.right = address_translator.axi_address

    with axi_dyn_mem.group("latch_write_en") as latch_write_en:
        write_en_reg.in_ = this_component["write_en"]
        write_en_reg.write_en = 1
        latch_write_en.done = write_en_reg.done
    
    #Control
    read_controller_invoke = invoke(
            axi_dyn_mem.get_cell(f"read_controller_{name}"),
            in_axi_address=base_addr_adder.out,
            in_ARESETn=this_component[f"ARESETn"],
            in_ARREADY=this_component[f"ARREADY"],
            in_RVALID=this_component[f"RVALID"],
            in_RLAST=this_component[f"RLAST"],
            in_RDATA=this_component[f"RDATA"],
            in_RRESP=this_component[f"RRESP"],
            out_ARVALID=this_component[f"ARVALID"],
            out_ARADDR=this_component[f"ARADDR"],
            out_ARSIZE=this_component[f"ARSIZE"],
            out_ARLEN=this_component[f"ARLEN"],
            out_ARBURST=this_component[f"ARBURST"],
            out_ARPROT=this_component[f"ARPROT"],
            out_RREADY=this_component[f"RREADY"],
            out_read_data=this_component[f"read_data"],
        )

    write_controller_invoke = invoke(
            axi_dyn_mem.get_cell(f"write_controller_{name}"),
            in_axi_address=base_addr_adder.out,
            in_write_data=this_component["write_data"],
            in_ARESETn=this_component["ARESETn"],
            in_AWREADY=this_component["AWREADY"],
            in_WREADY=this_component["WREADY"],
            in_BVALID=this_component["BVALID"],
            out_AWVALID=this_component["AWVALID"],
            out_AWADDR=this_component["AWADDR"],
            out_AWSIZE=this_component["AWSIZE"],
            out_AWLEN=this_component["AWLEN"],
            out_AWBURST=this_component["AWBURST"],
            out_AWPROT=this_component[f"AWPROT"],
            out_WVALID=this_component["WVALID"],
            out_WLAST=this_component["WLAST"],
            out_WDATA=this_component["WDATA"],
            out_BREADY=this_component["BREADY"],
    )
    
    axi_dyn_mem.control += [
        latch_write_en,
        if_(write_en_reg.out, write_controller_invoke, read_controller_invoke)
    ]

    

# NOTE: Unlike the channel functions, this can expect multiple mems
def add_wrapper_comp(prog, mems):

    add_control_subordinate(prog,mems)
    for mem in mems:
        add_address_translator(prog, mem)
        add_read_controller(prog, mem)
        add_write_controller(prog, mem)
        add_axi_dyn_mem(prog, mem)
    
    wrapper_comp = prog.component("Toplevel")
    wrapper_comp.attribute("toplevel", 1)
    # Get handles to be used later

    ref_mem_kwargs = {}

    # Create single main cell
    main_compute = wrapper_comp.comp_instance(
        "main_compute", "main", check_undeclared=False
    )

    # Generate XRT Control Ports for AXI Lite Control Subordinate,
    # must be prefixed with `s_axi_control`
    # This is copied from `axi_controller_generator.py`
    prefix = "s_axi_control_"
    wrapper_inputs = [
        (f"{prefix}AWVALID", 1),
        # XRT imposes a 16-bit address space for the control subordinate
        (f"{prefix}AWADDR", 16),
        # ("AWPROT", 3), #We don't do anything with this
        (f"{prefix}WVALID", 1),
        # Want to use 32 bits because the registers in XRT are asusemd to be this size
        (f"{prefix}WDATA", 32),
        # We don't use WSTRB but it is required by some versions of the spec. We should tie high on subordinate.
        (f"{prefix}WSTRB", int(32 / 8)),
        (f"{prefix}BREADY", 1),
        (f"{prefix}ARVALID", 1),
        (f"{prefix}ARADDR", 16),
        # ("ARPROT", 3), #We don't do anything with this
        (f"{prefix}RVALID", 1),
        (f"ap_rst_n", 1),
        (f"ap_clk", 1, ["clk"])
    ]

    wrapper_outputs = [
        (f"{prefix}AWREADY", 1),
        (f"{prefix}WREADY", 1),
        (f"{prefix}BVALID", 1),
        (f"{prefix}BRESP", 2),  
        (f"{prefix}ARREADY", 1),
        (f"{prefix}RREADY", 1),
        (f"{prefix}RDATA", 32),
        (f"{prefix}RRESP", 2),  
        ("ap_start", 1),
        ("ap_done", 1),
    ]

    add_comp_ports(wrapper_comp, wrapper_inputs, wrapper_outputs)
    
    if GENERATE_FOR_XILINX:
        control_subordinate = wrapper_comp.cell(f"control_subordinate", prog.get_component("control_subordinate"))
        ap_start_block_reg = wrapper_comp.reg(1, f"ap_start_block_reg")
        ap_done_reg = wrapper_comp.reg(1, f"ap_done_reg")

        with wrapper_comp.continuous:
            control_subordinate.ap_done_in = ap_done_reg.out


    #NOTE: This breaks encapsulation of modules a bit,
    # but allows us to block on ap_start in the control block without
    # adding new control flow constructs.

    # Ideally, it'd be nice to have this functionality included as part of
    # the control flow of the wrapper or perhaps the main_compute invocation? 
    with wrapper_comp.group(f"block_ap_start") as block_ap_start:
        ap_start_block_reg.in_ = 1
        ap_start_block_reg.write_en = control_subordinate.ap_start
        block_ap_start.done = ap_start_block_reg.done

    with wrapper_comp.group(f"assert_ap_done") as assert_ap_done:
        ap_done_reg.in_ = 1
        ap_done_reg.write_en = 1
        assert_ap_done.done = ap_done_reg.done

    # Generate manager controllers for each memory
    for mem in mems:
        mem_name = mem[name_key]
        # These input/output names in the toplevel (i.e. 'm_axi_A0_ARREADY') need to match
        # the kernel.xml file generated by `xml_generator.py`.
        # We add the prefix `m_axi_` to maintain compatibility with the old verilog wrapper.
        # Once we deprecate the old wrapper we can probably remove this prefix here and modify `xml_generator.py`
        prefixed_mem_name = f"m_axi_{mem[name_key]}"
        # Inputs/Outputs
        wrapper_inputs = [
            (f"{prefixed_mem_name}_ARREADY", 1),
            (f"{prefixed_mem_name}_RVALID", 1),
            (f"{prefixed_mem_name}_RLAST", 1),
            (f"{prefixed_mem_name}_RDATA", mem[width_key]),
            (f"{prefixed_mem_name}_RRESP", 2),
            (f"{prefixed_mem_name}_AWREADY", 1),
            (f"{prefixed_mem_name}_WREADY", 1),
            (f"{prefixed_mem_name}_BVALID", 1),
            # Only used for waveform tracing, not sent anywhere
            (f"{prefixed_mem_name}_BRESP", 2),
            # Only needed for coctb compatability, tied low
            (f"{prefixed_mem_name}_RID", 1),
        ]

        wrapper_outputs = [
            (f"{prefixed_mem_name}_ARVALID", 1),
            (f"{prefixed_mem_name}_ARADDR", 64),
            (f"{prefixed_mem_name}_ARSIZE", 3),
            (f"{prefixed_mem_name}_ARLEN", 8),
            (f"{prefixed_mem_name}_ARBURST", 2),
            (f"{prefixed_mem_name}_RREADY", 1),
            (f"{prefixed_mem_name}_AWVALID", 1),
            (f"{prefixed_mem_name}_AWADDR", 64),
            (f"{prefixed_mem_name}_AWSIZE", 3),
            (f"{prefixed_mem_name}_AWLEN", 8),
            (f"{prefixed_mem_name}_AWBURST", 2),
            (f"{prefixed_mem_name}_AWPROT", 3),
            (f"{prefixed_mem_name}_WVALID", 1),
            (f"{prefixed_mem_name}_WLAST", 1),
            (f"{prefixed_mem_name}_WDATA", mem[width_key]),
            (f"{prefixed_mem_name}_BREADY", 1),
            # ID signals are needed for cocotb compatability, tied low
            (f"{prefixed_mem_name}_ARID", 1),
            (f"{prefixed_mem_name}_AWID", 1),
            (f"{prefixed_mem_name}_WID", 1),
            (f"{prefixed_mem_name}_BID", 1),
        ]

        add_comp_ports(wrapper_comp, wrapper_inputs, wrapper_outputs)

        # Cells

        # TODO: Don't think these need to be marked external, but we
        # we need to raise them at some point form original calyx program
        axi_mem = wrapper_comp.cell(f"axi_dyn_mem_{mem_name}", prog.get_component(f"axi_dyn_mem_{mem_name}"))
        # Wires

        with wrapper_comp.continuous:
            # Tie IDs low, needed for cocotb compatability. Not used anywhere
            wrapper_comp.this()[f"{prefixed_mem_name}_ARID"] = 0
            wrapper_comp.this()[f"{prefixed_mem_name}_AWID"] = 0
            wrapper_comp.this()[f"{prefixed_mem_name}_WID"] = 0
            wrapper_comp.this()[f"{prefixed_mem_name}_BID"] = 0

            # Connect wrapper ports with axi_dyn_mem ports

            # Read controller portion inputs
            axi_mem["ARESETn"] = wrapper_comp.this()[f"ap_rst_n"] #note that both styles work
            # wrapper_comp.this()[f"{mem_name}_ARESETn"] = axi_mem["ARESETn"] #note that both styles work
            axi_mem.ARREADY = wrapper_comp.this()[f"{prefixed_mem_name}_ARREADY"]
            axi_mem.RVALID = wrapper_comp.this()[f"{prefixed_mem_name}_RVALID"]
            axi_mem.RLAST = wrapper_comp.this()[f"{prefixed_mem_name}_RLAST"]
            axi_mem.RDATA = wrapper_comp.this()[f"{prefixed_mem_name}_RDATA"]
            axi_mem.RRESP = wrapper_comp.this()[f"{prefixed_mem_name}_RRESP"]
            # Read controller outputs
            wrapper_comp.this()[f"{prefixed_mem_name}_ARVALID"] = axi_mem.ARVALID
            wrapper_comp.this()[f"{prefixed_mem_name}_ARADDR"] = axi_mem.ARADDR
            wrapper_comp.this()[f"{prefixed_mem_name}_ARSIZE"] = axi_mem.ARSIZE
            wrapper_comp.this()[f"{prefixed_mem_name}_ARLEN"] = axi_mem.ARLEN
            wrapper_comp.this()[f"{prefixed_mem_name}_ARBURST"] = axi_mem.ARBURST
            wrapper_comp.this()[f"{prefixed_mem_name}_RREADY"] = axi_mem.RREADY
            # Write controller inputs
            axi_mem.AWREADY = wrapper_comp.this()[f"{prefixed_mem_name}_AWREADY"]
            axi_mem.WREADY = wrapper_comp.this()[f"{prefixed_mem_name}_WREADY"]
            axi_mem.BVALID = wrapper_comp.this()[f"{prefixed_mem_name}_BVALID"]
            # Write controller outputs
            wrapper_comp.this()[f"{prefixed_mem_name}_AWVALID"] = axi_mem.AWVALID
            wrapper_comp.this()[f"{prefixed_mem_name}_AWADDR"] = axi_mem.AWADDR
            wrapper_comp.this()[f"{prefixed_mem_name}_AWSIZE"] = axi_mem.AWSIZE
            wrapper_comp.this()[f"{prefixed_mem_name}_AWLEN"] = axi_mem.AWLEN
            wrapper_comp.this()[f"{prefixed_mem_name}_AWBURST"] = axi_mem.AWBURST
            wrapper_comp.this()[f"{prefixed_mem_name}_AWPROT"] = axi_mem.AWPROT
            wrapper_comp.this()[f"{prefixed_mem_name}_WVALID"] = axi_mem.WVALID
            wrapper_comp.this()[f"{prefixed_mem_name}_WLAST"] = axi_mem.WLAST
            wrapper_comp.this()[f"{prefixed_mem_name}_WDATA"] = axi_mem.WDATA
            wrapper_comp.this()[f"{prefixed_mem_name}_BREADY"] = axi_mem.BREADY

            if GENERATE_FOR_XILINX:
                axi_mem["base_address"] = control_subordinate[f"{mem_name}_base_addr"]



        # Creates `<mem_name> = internal_mem_<mem_name>` as refs in invocation of `main_compute`
        ref_mem_kwargs[f"ref_{mem_name}"] = axi_mem

    # Control

    # Compute invoke
    # Assumes refs should be of form `<mem_name> = internal_mem_<mem_name>`
    main_compute_invoke = invoke(
        main_compute, **ref_mem_kwargs
    )
    control_subordinate_invoke = invoke(
        control_subordinate,
        in_ARESETn=wrapper_comp.this()[f"ap_rst_n"],
        in_AWVALID = wrapper_comp.this()[f"s_axi_control_AWVALID"],
        in_AWADDR = wrapper_comp.this()[f"s_axi_control_AWADDR"],
        in_WVALID = wrapper_comp.this()[f"s_axi_control_WVALID"],
        in_WDATA = wrapper_comp.this()[f"s_axi_control_WDATA"],
        in_WSTRB = wrapper_comp.this()[f"s_axi_control_WSTRB"],
        in_BREADY = wrapper_comp.this()[f"s_axi_control_BREADY"],
        in_ARVALID = wrapper_comp.this()[f"s_axi_control_ARVALID"],
        in_ARADDR = wrapper_comp.this()[f"s_axi_control_ARADDR"],
        in_RVALID = wrapper_comp.this()[f"s_axi_control_RVALID"],
        out_AWREADY = wrapper_comp.this()[f"s_axi_control_AWREADY"],
        out_WREADY = wrapper_comp.this()[f"s_axi_control_WREADY"],
        out_BVALID = wrapper_comp.this()[f"s_axi_control_BVALID"],
        out_BRESP = wrapper_comp.this()[f"s_axi_control_BRESP"],
        out_ARREADY = wrapper_comp.this()[f"s_axi_control_ARREADY"],
        out_RDATA = wrapper_comp.this()[f"s_axi_control_RDATA"],
        out_RREADY = wrapper_comp.this()[f"s_axi_control_RREADY"],
        out_RRESP = wrapper_comp.this()[f"s_axi_control_RRESP"],
        )



    # Compiler should reschedule these 2 seqs to be in parallel right?
    wrapper_comp.control += par(
        control_subordinate_invoke,
        [block_ap_start, main_compute_invoke, assert_ap_done]
        )


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
    check_mems_wellformed(mems)
    add_wrapper_comp(prog, mems)
    return prog.program


def check_mems_wellformed(mems):
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
        raise Exception("axi generator takes 1 yxi file name as argument")
    else:
        try:
            yxi_filename = sys.argv[1]
            if not yxi_filename.endswith(".yxi"):
                raise Exception("axi generator requires an yxi file")
        except:
            pass  # no arg passed
    with open(yxi_filename, "r", encoding="utf-8") as yxifile:
        yxifile = open(yxi_filename)
        yxi = json.load(yxifile)
        mems = yxi["memories"]
        build().emit()