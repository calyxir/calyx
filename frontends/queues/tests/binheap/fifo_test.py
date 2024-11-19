# pylint: disable=import-error
import sys
import calyx.builder as cb
import queues.queue_call as qc
from queues import binheap_fifo


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    fifo = binheap_fifo.insert_binheap_fifo(prog, "fifo")
    qc.insert_main(prog, fifo, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
