# pylint: disable=import-error
import sys
import fifo
import pifo
import calyx.builder as cb
import calyx.queue_call as qc


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    fifo_purple = fifo.insert_fifo(prog, "fifo_purple")
    fifo_tangerine = fifo.insert_fifo(prog, "fifo_tangerine")
    pifo_red = pifo.insert_pifo(prog, "pifo_red", fifo_purple, fifo_tangerine, 100)
    fifo_blue = fifo.insert_fifo(prog, "fifo_blue")
    pifo_root = pifo.insert_pifo(prog, "pifo_root", pifo_red, fifo_blue, 200)
    qc.insert_main(prog, pifo_root, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
