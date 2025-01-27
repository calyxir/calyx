# pylint: disable=import-error
import sys
import calyx.builder as cb
import queues.queue_call as qc
import queues.binheap.round_robin as rr
import queues.binheap.strict as st
import queues.tree as tr
import queues.flow_inference as fi

# This complex tree has the shape rr(strict(A, B, C), rr(D, E, F), strict(G, H))


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()

    fi_strict1 = fi.insert_boundary_flow_inference(prog, "fi_strict1", [44, 88, 133])
    pifo_strict1 = st.insert_binheap_strict(prog, "pifo_strict1", 3, [0, 1, 2], fi_strict1)

    fi_rr = fi.insert_boundary_flow_inference(prog, "fi_rr", [177, 221, 266])
    pifo_rr = rr.insert_binheap_rr(prog, "pifo_rr", 3, fi_rr)

    fi_strict2 = fi.insert_boundary_flow_inference(prog, "fi_strict2", [333, 400])
    pifo_strict2 = st.insert_binheap_strict(prog, "pifo_strict2", 2, [0, 1], fi_strict2)

    fi_root = fi.insert_value_flow_inference(prog, "fi_root", 3)
    pifo_root = rr.insert_binheap_rr(prog, "pifo_root", 3, fi_root)

    fi_tree = fi.insert_boundary_flow_inference(prog, "fi_tree", [133, 266, 400], flow_bits=32)
    pifo_tree = tr.insert_tree(prog, "pifo_tree", pifo_root, [pifo_strict1, pifo_rr, pifo_strict2], fi_tree)

    qc.insert_main(prog, pifo_tree, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
