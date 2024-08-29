import sys
import calyx.builder as cb
import queues.queue_call as qc
from queues.strict_or_rr import generate


if __name__ == "__main__":
    """Invoke the top-level function to build the program, with 3 flows."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv

    prog = cb.Builder()
    pifo = generate(prog, 3, True)
    qc.insert_main(prog, pifo, num_cmds, keepgoing=keepgoing)
    prog.program.emit()
