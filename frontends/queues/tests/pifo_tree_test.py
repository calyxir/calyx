# pylint: disable=import-error
import sys
import queues.fifo as fifo
import calyx.builder as cb
import queues.queue_call as qc
import queues.strict_or_rr as strict_or_rr


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    fifo_queue = fifo.insert_fifo(prog, "fifo")
    pifo_red = strict_or_rr.insert_queue(
        prog, "pifo_red", [fifo_queue, fifo_queue], [0, 100, 200], 2, [], True
    )
    pifo_root = strict_or_rr.insert_queue(
        prog, "pifo_root", [pifo_red, fifo_queue], [0, 200, 400], 2, [], True
    )
    qc.insert_main(prog, pifo_root, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
