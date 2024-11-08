import sys
import calyx.builder as cb
import queues.queue_call as qc
from queues import binheap_strict


if __name__ == "__main__":
    """Invoke the top-level function to build the program, with 6 flows."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv

    prog = cb.Builder()
    pifo = binheap_strict.generate(prog, 6)
    qc.insert_main(prog, pifo, num_cmds, keepgoing=keepgoing)
    prog.program.emit()
