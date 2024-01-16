from calyx.builder import Builder, const, add_comp_params, invoke, while_with, par
from calyx import py_ast as ast
from math import log2
import json

# In general, ports to the wrapper are uppercase, internal registers are lower case.

yxi_input = """
{
  "toplevel": "main",
  "memories": [
    {
      "name": "A0",
      "width": 32,
      "size": 8
    },
    {
      "name": "B0",
      "width": 32,
      "size": 8
    },
    {
      "name": "v0",
      "width": 32,
      "size": 1
    }
  ]
}
"""

yxi = json.loads(yxi_input)
mems = yxi["memories"]


def add_arread_channel(prog, mem):
    # Inputs/outputs
    arread_channel = prog.component("m_arread_channel")
    arread_inputs = [("ARESETn", 1), ("ARREADY", 1)]
    arread_outputs = [
        ("ARVALID", 1),
        ("ARADDR", 64),
        ("ARSIZE", 3),  # bytes used in transfer
        ("ARLEN", 8),  # number of transfers in transaction
        ("ARBURST", 2),  # for XRT should be tied to 2'b01 for WRAP burst
        ("ARPROT", 3),  # tied to be priviliged, nonsecure, data access request
    ]
    add_comp_params(arread_channel, arread_inputs, arread_outputs)

    # Cells
    arvalid = arread_channel.reg("arvalid", 1)
    arvalid_was_high = arread_channel.reg("arvalid_was_high", 1)
    base_addr = arread_channel.reg("base_addr", 64, is_ref=True)
    arlen = arread_channel.reg("arlen", 8)

    # Number of txns we want to occur before m_arread_channel is done
    # TODO: parameterize
    txn_n = arread_channel.const("txn_n", 32, 1)
    txn_count = arread_channel.reg("txn_count", 32)
    txn_adder = arread_channel.add(32, "txn_adder")

    # Need to put block_transfer register here to avoid combinational loops
    bt_reg = arread_channel.reg("bt_reg", 1)

    # Wires
    with arread_channel.continuous:
        arread_channel.this().ARVALID = arvalid.out

    # Groups

    # Responsible for asserting ARVALID, and deasserting it a cycle after the handshake.
    # This is necesarry because of the way transitions between groups work. See #1828 https://github.com/calyxir/calyx/issues/1828
    with arread_channel.group("do_ar_transfer") as do_ar_transfer:
        ARREADY = arread_channel.this().ARREADY
        # TODO: Can we simplify this? See comments #1846 https://github.com/calyxir/calyx/pull/1846
        # Assert arvalid if it was not previously high
        arvalid.in_ = ~arvalid_was_high.out @ 1
        # Deassert in the next cycle once it is high
        arvalid.in_ = (arvalid.out & ARREADY & arvalid_was_high.out) @ 0
        arvalid.write_en = 1

        arvalid_was_high.in_ = (~(arvalid.out & ARREADY) & ~arvalid_was_high.out) @ 1
        arvalid_was_high.write_en = (
            ~(arvalid.out & ARREADY) & ~arvalid_was_high.out
        ) @ 1

        # Drive output signals for transfer
        arread_channel.this().ARADDR = base_addr.out
        # This is taken from mem size, we assume the databus width is the size of our memory cell
        # TODO(nathanielnrn): convert to binary instead of decimal
        arread_channel.this().ARSIZE = width_arsize(mem["width"])
        # TODO(nathanielnrn): Figure our how to set arlen. For now set to size of mem. A
        arread_channel.this().ARLEN = arlen.out
        arread_channel.this().ARBURST = 1  # Must be INCR for XRT
        # Required by spec, we hardcode to privileged, non-secure, data access
        arread_channel.this().ARPROT = 0b110

        # control block_transfer reg to go low after one cycle
        bt_reg.in_ = (ARREADY & arvalid.out) @ 1
        bt_reg.in_ = ~(ARREADY & arvalid.out) @ 0
        bt_reg.write_en = 1
        do_ar_transfer.done = bt_reg.out

        # TODO(nathanielnrn): Continue adding cells. Beforehand need to make sure the calyx wrapper can interface with XRT shell.

    with arread_channel.group("incr_txn_count") as incr_txn_count:
        txn_adder.left = txn_count.out
        txn_adder.right = 1
        txn_count.in_ = txn_adder.out
        txn_count.write_en = 1
        incr_txn_count.done = txn_count.done

    # check if txn_count == txn_n

    check_reads_done = arread_channel.neq_use(
        txn_count.out, txn_n.out, signed=False, cellname="perform_reads", width=32
    )
    # with arread_channel.comb_group("check_reads_done") as check_reads_done:
    #     perform_reads.left = txn_count.out
    #     perform_reads.right = txn_n.out

    invoke_txn_count = invoke(txn_count, in_in=0)
    # ARLEN must be between 0-255, make sure to subtract 1 from yxi size when assigning to ARLEN
    assert mem["size"] < 256, "Memory size must be less than 256"
    invoke_arlen = invoke(arlen, in_in=mem["size"] - 1)

    while_body = [
        par(
            invoke(bt_reg, in_in=0),
            invoke(arvalid_was_high, in_in=0),
        ),
        do_ar_transfer,
        invoke(arvalid, in_in=0),
        incr_txn_count,
    ]

    while_loop = while_with(check_reads_done, while_body)
    arread_channel.control += [invoke_txn_count, invoke_arlen, while_loop]


def build():
    prog = Builder()
    add_arread_channel(prog, mems[0])
    return prog.program


def width_in_bytes(width: int):
    assert width % 8 == 0, "Width must be a multiple of 8."
    return width // 8


def width_arsize(width: int):
    log = log2(width_in_bytes(width))
    assert log.is_integer(), "Width must be a power of 2."
    return int(log)


if __name__ == "__main__":
    build().emit()
