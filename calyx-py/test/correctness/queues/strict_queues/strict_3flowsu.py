# pylint: disable=import-error
import os
import sys
import inspect

currentdir = os.path.dirname(os.path.abspath(inspect.getfile(inspect.currentframe())))
parentdir = os.path.dirname(currentdir)
sys.path.insert(0, parentdir)
import fifo
import calyx.builder as cb
import calyx.queue_call as qc
import strict

# This determines the maximum possible length of the queue:
# The max length of the queue will be 2^QUEUE_LEN_FACTOR.
QUEUE_LEN_FACTOR = 4

def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    numflows = 3
    sub_fifos = []
    for n in range(numflows):
        name = "fifo" + str(n)
        sub_fifo = fifo.insert_fifo(prog, name, QUEUE_LEN_FACTOR)
        sub_fifos.append(sub_fifo)

    pifo = strict.insert_strict_pifo(prog, "pifo", sub_fifos, [0, 133, 266, 400], numflows, [1, 2, 0])
    qc.insert_main(prog, pifo, 20)
    return prog.program


if __name__ == "__main__":
    build().emit()