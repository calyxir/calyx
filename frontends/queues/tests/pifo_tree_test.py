# pylint: disable=import-error
import sys
import queues.fifo as fifo
import calyx.builder as cb
import queues.queue_call as qc
import queues.strict_or_rr as strict_or_rr
import queues.flow_inference as fi


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()

    fifo_queue = fifo.insert_fifo(prog, "fifo")

    flow_infer_red = fi.insert_boundary_flow_inference(
        prog, "flow_infer_red", [100, 200]
    )
    pifo_red = strict_or_rr.insert_queue(
        prog, "pifo_red", True, [fifo_queue, fifo_queue], flow_infer_red
    )

    flow_infer_root = fi.insert_boundary_flow_inference(
        prog, "flow_infer_root", [200, 400]
    )
    pifo_root = strict_or_rr.insert_queue(
        prog, "pifo_root", True, [pifo_red, fifo_queue], flow_infer_root
    )

    qc.insert_main(prog, pifo_root, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
