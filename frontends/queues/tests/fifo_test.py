# pylint: disable=import-error
import sys
import calyx.builder as cb
import queues.queue_call as qc
import queues.fifo as fifo


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    queue = fifo.insert_fifo(prog, "fifo")
    qc.insert_main(prog, queue, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
