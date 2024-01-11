from calyx.builder import Builder, const, add_comp_params
from calyx import py_ast as ast


def add_arread_channel(prog):
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

    arvalid = arread_channel.reg("arvalid", 1)
    arvalid_was_high = arread_channel.reg("arvalid_was_high", 1)
    base_addr = arread_channel.reg("base_addr", 64, is_ref=True)
    # TODO(nathanielnrn): Continue adding cells. Beforehand need to make sure the calyx wrapper can interface with XRT shell.
