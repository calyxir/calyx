from calyx.builder import Builder, const, add_comp_params
from calyx import py_ast as ast

# In general, ports to the wrapper are uppercase, internal registers are lower case.


def add_arread_channel(prog):
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
    # TODO: parameterie
    txn_n = arread_channel.const("txn_n", 32, 1)
    txn_count = arread_channel.reg("txn_count", 32)
    perform_reads = arread_channel.neq(32, "perform_reads")
    txn_adder = arread_channel.add(32, "txn_adder")

    # Need to put block_transfer register here to avoid combinational loops
    bt_reg = arread_channel.reg("bt_reg", 1)

    # Wires
    with arread_channel.continuous:
        arread_channel.this().ARVALID = arvalid.out

    # Groups
    # TODO(nathanielnrn): Do we need this group explicitly? Can we just use an invoke?
    with arread_channel.group("deassert_arvalid") as deassert_arvalid:
        arvalid.in_ = 1
        arvalid.write_en = 1
        deassert_arvalid.done = arvalid.done

    with arread_channel.group("reset_bt") as reset_bt:
        bt_reg.in_ = 0
        bt_reg.write_en = 1
        reset_bt.done = bt_reg.done

    with arread_channel.group("reset_was_high") as reset_was_high:
        arvalid_was_high.in_ = 0
        arvalid_was_high.write_en = 1
        reset_was_high.done = arvalid_was_high.done

    # Responsible for asserting ARVALID, and deasserting it a cycle after the handshake.
    # This is necesarry because of the way transitions between groups work. See #1828 https://github.com/calyxir/calyx/issues/1828
    with arread_channel.grouo("do_ar_transfer") as do_ar_transfer:
        ARREADY = arread_channel.this().ARREADY
        # TODO: Can we simplify this? See comments #1846 https://github.com/calyxir/calyx/pull/1846
        # Assert arvalid if it was not previously high
        arvalid.in_ = ~(arvalid.out & ARREADY) & ~arvalid_was_high.out @ 1
        # Deassert in the next cycle once it is high
        arvalid.in_ = arvalid.out & ARREADY & arvalid_was_high.out @ 0
        arvalid.write_en = 1

        arvalid_was_high.in_ = ~(arvalid.out & ARREADY) & ~arvalid_was_high.out @ 1
        arvalid_was_high.write_en = ~(arvalid.out & ARREADY) & ~arvalid_was_high.out @ 1

    # TODO(nathanielnrn): Continue adding cells. Beforehand need to make sure the calyx wrapper can interface with XRT shell.


def build():
    prog = Builder()
    add_arread_channel(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
