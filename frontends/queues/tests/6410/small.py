# pylint: disable=import-error
import sys
import calyx.builder as cb
import queues.queue_call as qc
import queues.strict_or_rr as strict_or_rr
import queues.fifo as fifo
import queues.flow_inference as fi

# New target shape:
#      rr
#    /    \
#  sp      rr
# (A,B)   (C,D)


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()

    # Leaf FIFOs A, B, C, D
    fifo_A = fifo.insert_fifo(prog, "fifo_A", 7)
    fifo_B = fifo.insert_fifo(prog, "fifo_B", 7)
    fifo_C = fifo.insert_fifo(prog, "fifo_C", 7)
    fifo_D = fifo.insert_fifo(prog, "fifo_D", 7)

    # Flow inference nodes for the new structure
    fi_sp = fi.insert_boundary_flow_inference(
        prog, "fi_sp", [100, 200]
    )  # priorities for sp(A,B)

    fi_rr2 = fi.insert_boundary_flow_inference(
        prog, "fi_rr2", [300, 400]
    )  # priorities for rr(C,D)

    fi_root = fi.insert_boundary_flow_inference(
        prog, "fi_root", [200, 400]
    )  # priorities for rr(sp, rr2)

    # sp(A, B): strict priority queue
    pifo_sp = strict_or_rr.insert_queue(
        prog,
        "pifo_sp",
        False,
        [fifo_A, fifo_B],
        fi_sp,
        [0, 1],
        7,  # strict priority
    )

    # rr(C, D): round-robin queue
    pifo_rr2 = strict_or_rr.insert_queue(
        prog, "pifo_rr2", True, [fifo_C, fifo_D], fi_rr2, 7
    )

    # Root: rr(sp(A,B), rr(C,D))
    pifo_root = strict_or_rr.insert_queue(
        prog, "pifo_root", True, [pifo_sp, pifo_rr2], fi_root, 7
    )

    # Main block
    qc.insert_main(prog, pifo_root, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
