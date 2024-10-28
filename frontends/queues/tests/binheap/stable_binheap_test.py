# pylint: disable=import-error
import sys
import calyx.builder as cb
import queues.queue_call as qc
import queues.binheap.stable_binheap as sbh


if __name__ == "__main__":
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv

    prog = cb.Builder()

    binheap = sbh.insert_stable_binheap(prog, "stable_binheap")
    qc.insert_main(prog, binheap, num_cmds, keepgoing=keepgoing, use_ranks=True)

    prog.program.emit()
