# pylint: disable=import-error
import sys
import calyx.builder as cb
import queues.queue_call as qc
from queues import binheap_pifo


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    pifo = binheap_pifo.insert_binheap_pifo(prog, "pifo")
    qc.insert_main(prog, pifo, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
