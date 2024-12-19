# pylint: disable=import-error
import sys
import calyx.builder as cb
import queues.queue_call as qc
import queues.binheap.pifo as bhp


if __name__ == "__main__":
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv

    prog = cb.Builder()

    pifo = bhp.insert_binheap_pifo(prog, "pifo")
    qc.insert_main(prog, pifo, num_cmds, keepgoing=keepgoing)

    prog.program.emit()
