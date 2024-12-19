# pylint: disable=import-error
import sys
import calyx.builder as cb
import queues.queue_call as qc
import queues.binheap.fifo as bhf


if __name__ == "__main__":
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv

    prog = cb.Builder()

    fifo = bhf.insert_binheap_fifo(prog, "fifo")
    qc.insert_main(prog, fifo, num_cmds, keepgoing=keepgoing)

    prog.program.emit()
