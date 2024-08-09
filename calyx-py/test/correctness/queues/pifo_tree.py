# pylint: disable=import-error
import sys
import fifo
import calyx.builder as cb
import calyx.queue_call as qc
import strict_and_rr_queues.gen_strict_or_rr as strict_or_rr


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    fifo_purple = fifo.insert_fifo(prog, "fifo_purple")
    fifo_tangerine = fifo.insert_fifo(prog, "fifo_tangerine")
    pifo_red = strict_or_rr.insert_queue(
        prog, "pifo_red", [fifo_purple, fifo_tangerine], [0, 100, 200], 2, [], True
    )
    fifo_blue = fifo.insert_fifo(prog, "fifo_blue")
    pifo_root = strict_or_rr.insert_queue(
        prog, "pifo_root", [pifo_red, fifo_blue], [0, 200, 400], 2, [], True
    )
    qc.insert_main(prog, pifo_root, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
