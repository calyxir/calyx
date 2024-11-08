import sys
import calyx.builder as cb
import queues.queue_call as qc
from queues import binheap_rr


if __name__ == "__main__":
    """Invoke the top-level function to build the program, with 3 flows."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv

    prog = cb.Builder()
    pifo = binheap_rr.generate(prog, 3)
    qc.insert_main(prog, pifo, num_cmds, keepgoing=keepgoing)
    prog.program.emit()
