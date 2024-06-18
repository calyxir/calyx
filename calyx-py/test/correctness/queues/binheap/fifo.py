# pylint: disable=import-error
import sys
import calyx.builder as cb
import calyx.queue_call as qc
from stable_binheap import insert_stable_binheap


def insert_binheap_fifo(prog, name, factor):
    """Inserts the component `fifo` into the program.

    It is implemented via a binary heap

    It has:
    - two inputs, `cmd` and `value`.
    - one memory, `mem`, of size 10.
    - two registers, `next_write` and `next_read`.
    - two ref registers, `ans` and `err`.
    """
    comp = prog.component(name)

    binheap = insert_stable_binheap(prog, "binheap", factor)
    binheap = comp.cell("binheap", binheap)

    cmd = comp.input("cmd", 2)
    value = comp.input("value", 32)

    ans = comp.reg(32, "ans", is_ref=True)
    err = comp.reg(1, "err", is_ref=True)

    comp.control += [
        cb.invoke(binheap, in_value=value, in_rank=cb.const(32, 1), in_cmd=cmd, ref_ans=ans, ref_err=err),
    ]

    return comp


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    prog = cb.Builder()
    fifo = insert_binheap_fifo(prog, "fifo", 6)
    qc.insert_main(prog, fifo, num_cmds)
    return prog.program


if __name__ == "__main__":
    build().emit()
