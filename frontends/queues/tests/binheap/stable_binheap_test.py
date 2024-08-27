# pylint: disable=import-error
import sys
import calyx.builder as cb
import tests.queue_call as qc
from queues.binheap.stable_binheap import insert_stable_binheap


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    binheap = insert_stable_binheap(prog, "binheap")
    qc.insert_main(prog, binheap, num_cmds, keepgoing=keepgoing, use_ranks=True)
    return prog.program


if __name__ == "__main__":
    build().emit()
