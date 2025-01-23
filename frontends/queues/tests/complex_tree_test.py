# pylint: disable=import-error
import sys
import calyx.builder as cb
import queues.queue_call as qc
import queues.strict_or_rr as strict_or_rr
import queues.fifo as fifo
import queues.flow_inference as fi

# This complex tree has the shape rr(strict(A, B, C), rr(D, E, F), strict(G, H))


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()

    fifo_queue = fifo.insert_fifo(prog, "fifo")

    fi_strict1 = fi.insert_boundary_flow_inference(prog, "fi_strict1", [44, 88, 133])
    pifo_strict1 = strict_or_rr.insert_queue(
        prog,
        "pifo_strict1",
        False,
        [fifo_queue, fifo_queue, fifo_queue],
        fi_strict1,
        order=[0, 1, 2],
    )

    fi_rr = fi.insert_boundary_flow_inference(prog, "fi_rr", [177, 221, 266])
    pifo_rr = strict_or_rr.insert_queue(
        prog, "pifo_rr", True, [fifo_queue, fifo_queue, fifo_queue], fi_rr
    )

    fi_strict2 = fi.insert_boundary_flow_inference(prog, "fi_strict2", [333, 400])
    pifo_strict2 = strict_or_rr.insert_queue(
        prog, "pifo_strict2", False, [fifo_queue, fifo_queue], fi_strict2, order=[0, 1]
    )

    fi_root = fi.insert_boundary_flow_inference(prog, "fi_root", [133, 266, 400])
    pifo_root = strict_or_rr.insert_queue(
        prog, "pifo_root", True, [pifo_strict1, pifo_rr, pifo_strict2], fi_root
    )

    qc.insert_main(prog, pifo_root, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
