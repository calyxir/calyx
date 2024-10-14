# pylint: disable=import-error
import sys
import calyx.builder as cb
import queues.queue_call as qc
import queues.strict_or_rr as strict_or_rr
import queues.fifo as fifo

# This complex tree has the shape rr(strict(A, B, C), rr(D, E, F), strict(G, H))


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()

    fifo_queue = fifo.insert_fifo(prog, "fifo")

    pifo_strict1 = strict_or_rr.insert_queue(
        prog,
        "pifo_strict1",
        [fifo_queue, fifo_queue, fifo_queue],
        [0, 44, 88, 133],
        3,
        [0, 1, 2],
        False,
    )
    pifo_rr = strict_or_rr.insert_queue(
        prog,
        "pifo_rr",
        [fifo_queue, fifo_queue, fifo_queue],
        [133, 177, 221, 266],
        3,
        [0, 1, 2],
        True,
    )
    pifo_strict2 = strict_or_rr.insert_queue(
        prog,
        "pifo_strict2",
        [fifo_queue, fifo_queue],
        [266, 333, 400],
        2,
        [0, 1],
        False,
    )
    pifo_root = strict_or_rr.insert_queue(
        prog,
        "pifo_root",
        [pifo_strict1, pifo_rr, pifo_strict2],
        [0, 133, 266, 400],
        3,
        [],
        True,
    )

    qc.insert_main(prog, pifo_root, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
