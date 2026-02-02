from calyx.builder import (
    Builder,
    add_comp_ports,
    invoke,
    while_with,
    par,
    while_,
)
from typing import Literal
from math import log2, ceil
import json
import sys

handled_mems = []


def add_arread_channel(prog, mwidth, mlen):
    _add_m_to_s_address_channel(prog, mwidth, mlen, "AR")


def add_awwrite_channel(prog, mwidth, mlen):
    aw_channel = _add_m_to_s_address_channel(prog, mwidth, mlen, "AW")
    max_transfers = aw_channel.reg(8, "max_transfers", is_ref=True)

    # TODO(nathanielnrn): We eventually want to move beyond
    # the implicit 1 transaction that is the size of the memory
    # How should we store this?
    # Recall this goes to write channel as number of transfers it expectes to do before
    # setting WLAST high
    with aw_channel.get_group("do_aw_transfer"):
        max_transfers.in_ = mlen - 1
        max_transfers.write_en = 1


def _add_m_to_s_address_channel(prog, mwidth, mlen, prefix: Literal["AW", "AR"]):
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
    # Inputs/outputs
    m_to_s_address_channel = prog.component(f"m_{lc_x}_channel_{mwidth}_{mlen}")
    channel_inputs = [("ARESETn", 1), (f"{x}READY", 1)]
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
    curr_addr_axi = m_to_s_address_channel.reg(64, "curr_addr_axi", is_ref=True)
    xlen = m_to_s_address_channel.reg(8, f"{lc_x}len")

    # Number of txns we want to occur before m_arread_channel is done
    # TODO: parameterize
    txn_n = m_to_s_address_channel.const("txn_n", 32, 1)
    txn_count = m_to_s_address_channel.reg(32, "txn_count")
    txn_adder = m_to_s_address_channel.add(32, "txn_adder")

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
        m_to_s_address_channel.this()[f"{x}ADDR"] = curr_addr_axi.out
        # This is taken from mem size, we assume the databus width is the size
        # of our memory cell and that width is a power of 2
        # TODO(nathanielnrn): convert to binary instead of decimal
        m_to_s_address_channel.this()[f"{x}SIZE"] = width_xsize(mwidth)
        # TODO(nathanielnrn): Figure our how to set arlen. For now set to size of mem.
        m_to_s_address_channel.this()[f"{x}LEN"] = xlen.out
        m_to_s_address_channel.this()[f"{x}BURST"] = 1  # Must be INCR for XRT
        # Required by spec, we hardcode to privileged, non-secure, data access
        m_to_s_address_channel.this()[f"{x}PROT"] = 0b110

        # control block_transfer reg to go low after one cycle
        bt_reg.in_ = (xREADY & xvalid.out) @ 1
        bt_reg.in_ = ~(xREADY & xvalid.out) @ 0
        bt_reg.write_en = 1
        do_x_transfer.done = bt_reg.out

    with m_to_s_address_channel.group("incr_txn_count") as incr_txn_count:
        txn_adder.left = txn_count.out
        txn_adder.right = 1
        txn_count.in_ = txn_adder.out
        txn_count.write_en = 1
        incr_txn_count.done = txn_count.done

    # Control
    # check if txn_count == txn_n
    cellname = "perform_reads" if prefix == "AR" else "perform_writes"
    check_transactions_done = m_to_s_address_channel.neq_use(
        txn_count.out, txn_n.out, signed=False, cellname=cellname, width=32
    )

    invoke_txn_count = invoke(txn_count, in_in=0)
    # ARLEN must be between 0-255, make sure to subtract 1 from yxi
    # size when assigning to ARLEN
    assert mlen < 256, "Memory size must be less than 256"
    invoke_xlen = invoke(xlen, in_in=mlen - 1)

    while_body = [
        par(
            invoke(bt_reg, in_in=0),
            invoke(xhandshake_occurred, in_in=0),
        ),
        do_x_transfer,
        invoke(xvalid, in_in=0),
        incr_txn_count,
    ]

    while_loop = while_with(check_transactions_done, while_body)
    m_to_s_address_channel.control += [invoke_txn_count, invoke_xlen, while_loop]
    return m_to_s_address_channel


def add_read_channel(prog, mwidth, mlen):
    # Inputs/Outputs
    read_channel = prog.component("m_read_channel_{}_{}".format(mwidth, mlen))
    # TODO(nathanielnrn): We currently assume RDATA is the same width as the
    # memory. This limits throughput many AXI data busses are much wider
    # i.e., 512 bits.
    channel_inputs = [
        ("ARESETn", 1),
        ("RVALID", 1),
        ("RLAST", 1),
        ("RDATA", mwidth),
        ("RRESP", 2),
    ]
    channel_outputs = [("RREADY", 1)]
    add_comp_ports(read_channel, channel_inputs, channel_outputs)

    # Cells

    # We assume idx_size is exactly clog2(len). See comment in #1751
    # https://github.com/calyxir/calyx/issues/1751#issuecomment-1778360566

    mem_ref = read_channel.seq_mem_d1(
        name="mem_ref",
        bitwidth=mwidth,
        len=mlen,
        idx_size=clog2_or_1(mlen),
        is_external=False,
        is_ref=True,
    )

    # according to zipcpu, rready should be registered
    rready = read_channel.reg(1, "rready")
    curr_addr_internal_mem = read_channel.reg(
        clog2_or_1(mlen), "curr_addr_internal_mem", is_ref=True
    )
    curr_addr_axi = read_channel.reg(64, "curr_addr_axi", is_ref=True)
    # Registed because RLAST is high with laster transfer, not after
    # before this we were terminating immediately with
    # last transfer and not servicing it
    n_RLAST = read_channel.reg(1, "n_RLAST")
    # Stores data we want to write to our memory at end of block_transfer group
    read_data_reg = read_channel.reg(mwidth, "read_data_reg")

    bt_reg = read_channel.reg(1, "bt_reg")

    # Groups
    with read_channel.continuous:
        read_channel.this()["RREADY"] = rready.out

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
        read_data_reg.in_ = RDATA
        read_data_reg.write_en = (rready.out & RVALID) @ 1
        read_data_reg.write_en = ~(rready.out & RVALID) @ 0

        n_RLAST.in_ = RLAST @ 0
        n_RLAST.in_ = ~RLAST @ 1
        n_RLAST.write_en = 1

        # We are done after handshake
        bt_reg.in_ = (rready.out & RVALID) @ 1
        bt_reg.in_ = ~(rready.out & RVALID) @ 0
        bt_reg.write_en = 1
        block_transfer.done = bt_reg.out

    with read_channel.group("service_read_transfer") as service_read_transfer:
        # not ready till done servicing
        rready.in_ = 0
        rready.write_en = 1

        # write data we received to mem_ref
        mem_ref.addr0 = curr_addr_internal_mem.out
        mem_ref.write_data = read_data_reg.out
        mem_ref.write_en = 1
        mem_ref.content_en = 1
        service_read_transfer.done = mem_ref.done

    # creates group that increments curr_addr_internal_mem by 1. Creates adder and wires up correctly
    curr_addr_internal_mem_incr = read_channel.incr(curr_addr_internal_mem, 1)
    # TODO(nathanielnrn): Currently we assume that width is a power of 2 due to xSIZE.
    # In the future we should allow for non-power of 2 widths, will need some
    # splicing for this.
    # See https://cucapra.slack.com/archives/C05TRBNKY93/p1705587169286609?thread_ts=1705524171.974079&cid=C05TRBNKY93 # noqa: E501
    curr_addr_axi_incr = read_channel.incr(curr_addr_axi, width_in_bytes(mwidth))

    # Control
    invoke_n_RLAST = invoke(n_RLAST, in_in=1)
    invoke_bt_reg = invoke(bt_reg, in_in=0)
    while_body = [
        invoke_bt_reg,
        block_transfer,
        service_read_transfer,
        par(curr_addr_internal_mem_incr, curr_addr_axi_incr),
    ]
    while_n_RLAST = while_(n_RLAST.out, while_body)

    read_channel.control += [invoke_n_RLAST, while_n_RLAST]


def add_write_channel(prog, mwidth, mlen):
    # Inputs/Outputs
    write_channel = prog.component("m_write_channel_{}_{}".format(mwidth, mlen))
    channel_inputs = [("ARESETn", 1), ("WREADY", 1)]
    # TODO(nathanielnrn): We currently assume WDATA is the same width as the
    # memory. This limits throughput many AXI data busses are much wider
    # i.e., 512 bits.
    channel_outputs = [
        ("WVALID", 1),
        ("WLAST", 1),
        ("WDATA", mwidth),
    ]
    add_comp_ports(write_channel, channel_inputs, channel_outputs)

    # Cells
    # We assume idx_size is exactly clog2(len). See comment in #1751
    # https://github.com/calyxir/calyx/issues/1751#issuecomment-1778360566
    mem_ref = write_channel.seq_mem_d1(
        name="mem_ref",
        bitwidth=mwidth,
        len=mlen,
        idx_size=clog2_or_1(mlen),
        is_external=False,
        is_ref=True,
    )

    # according to zipcpu, rready should be registered
    wvalid = write_channel.reg(1, "wvalid")
    w_handshake_occurred = write_channel.reg(1, "w_handshake_occurred")
    # internal calyx memory indexing
    curr_addr_internal_mem = write_channel.reg(
        clog2_or_1(mlen), "curr_addr_internal_mem", is_ref=True
    )
    # host indexing, must be 64 bits
    curr_addr_axi = write_channel.reg(64, "curr_addr_axi", is_ref=True)

    curr_transfer_count = write_channel.reg(8, "curr_transfer_count")
    # Number of transfers we want to do in current txn
    max_transfers = write_channel.reg(8, "max_transfers", is_ref=True)

    # Register because w last is high with last transfer. Before this
    # We were terminating immediately with last transfer and not servicing it.
    n_finished_last_transfer = write_channel.reg(1, "n_finished_last_transfer")

    bt_reg = write_channel.reg(1, "bt_reg")

    # Groups
    with write_channel.continuous:
        write_channel.this()["WVALID"] = wvalid.out
        # Needed due to default assignment bug #1930
        mem_ref.write_en = 0

    with write_channel.group("service_write_transfer") as service_write_transfer:
        WREADY = write_channel.this()["WREADY"]

        # Assert then deassert. Can maybe getgit right of w_handshake_occurred in guard
        wvalid.in_ = (~(wvalid.out & WREADY) & ~w_handshake_occurred.out) @ 1
        wvalid.in_ = ((wvalid.out & WREADY) | w_handshake_occurred.out) @ 0
        wvalid.write_en = 1

        # Set high when wvalid is high even once
        # This is just wavlid.in_ guard from above
        # TODO: confirm this is correct?
        w_handshake_occurred.in_ = (wvalid.out & WREADY) @ 1
        w_handshake_occurred.in_ = ~(wvalid.out & WREADY) @ 0
        w_handshake_occurred.write_en = (~w_handshake_occurred.out) @ 1

        # Set data output based on intermal memory output
        mem_ref.addr0 = curr_addr_internal_mem.out
        mem_ref.content_en = 1
        write_channel.this()["WDATA"] = mem_ref.read_data

        write_channel.this()["WLAST"] = (
            max_transfers.out == curr_transfer_count.out
        ) @ 1
        write_channel.this()["WLAST"] = (
            max_transfers.out != curr_transfer_count.out
        ) @ 0

        # set high when WLAST is high and a handshake occurs
        n_finished_last_transfer.in_ = (
            (max_transfers.out == curr_transfer_count.out) & (wvalid.out & WREADY)
        ) @ 0
        n_finished_last_transfer.write_en = (
            (max_transfers.out == curr_transfer_count.out) & (wvalid.out & WREADY)
        ) @ 1

        # done after handshake
        bt_reg.in_ = (wvalid.out & WREADY) @ 1
        bt_reg.in_ = ~(wvalid.out & WREADY) @ 0
        bt_reg.write_en = 1
        service_write_transfer.done = bt_reg.out

        # Creates adder and wires up correctly
        curr_addr_internal_mem_incr = write_channel.incr(curr_addr_internal_mem, 1)
        # TODO(nathanielnrn): Currently we assume that width is a power of 2.
        # In the future we should allow for non-power of 2 widths, will need some
        # splicing for this.
        # See https://cucapra.slack.com/archives/C05TRBNKY93/p1705587169286609?thread_ts=1705524171.974079&cid=C05TRBNKY93 # noqa: E501
        curr_addr_axi_incr = write_channel.incr(curr_addr_axi, ceil(mwidth / 8))
        curr_transfer_count_incr = write_channel.incr(curr_transfer_count, 1)

        # Control
        init_curr_addr_internal_mem = invoke(curr_addr_internal_mem, in_in=0)
        init_n_finished_last_transfer = invoke(n_finished_last_transfer, in_in=1)
        while_n_finished_last_transfer_body = [
            invoke(bt_reg, in_in=0),
            service_write_transfer,
            par(
                curr_addr_internal_mem_incr,
                curr_transfer_count_incr,
                curr_addr_axi_incr,
                invoke(w_handshake_occurred, in_in=0),
            ),
        ]
        while_n_finished_last_transfer = while_(
            n_finished_last_transfer.out, while_n_finished_last_transfer_body
        )
        write_channel.control += [
            init_curr_addr_internal_mem,
            init_n_finished_last_transfer,
            while_n_finished_last_transfer,
        ]


# For now we assume all responses are OKAY because we don't have any error
# handling logic. So basically this sets BREADY high then lowers it on
# handshake.
def add_bresp_channel(prog, mwidth, mlen):
    # Inputs/Outputs
    bresp_channel = prog.component("m_bresp_channel_{}_{}".format(mwidth, mlen))
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
