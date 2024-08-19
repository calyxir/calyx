# pylint: disable=import-error
import sys
import calyx.builder as cb
import calyx.queue_call as qc
from stable_binheap import insert_stable_binheap

FACTOR = 4


def insert_binheap_fifo(prog, name, queue_size_factor=FACTOR):
    """Inserts the component `fifo` into the program.
    
    It is a first in, first out queue implemented via binary heap.

    It has:
    - two inputs, `cmd` and `value`.
        - `cmd` has width 1.
        - `value` has width 32.
    - one memory, `mem`, of size `2**queue_size_factor`.
    - two ref registers, `ans` and `err`.
        - `ans` has width 32.
        - `err` has width 1.

    We use `stable_binheap`, a binary heap that resolves ties based on insertion order. 
    That is, If two elements with the same rank are inserted, the element that was inserted 
    first is the one that gets popped first.

    Hence, we insert all elements with the same rank into `stable_binheap`, as our 
    tie-breaking mechanism ensures FIFO order. 
    """
    comp = prog.component(name)

    binheap = insert_stable_binheap(prog, "binheap", queue_size_factor)
    binheap = comp.cell("binheap", binheap)

    cmd = comp.input("cmd", 1)
    value = comp.input("value", 32)

    ans = comp.reg(32, "ans", is_ref=True)
    err = comp.reg(1, "err", is_ref=True)

    comp.control += [
        cb.invoke(
            binheap,
            in_value=value,
            in_rank=cb.const(32, 1),
            in_cmd=cmd,
            ref_ans=ans,
            ref_err=err,
        )
    ]

    return comp


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    fifo = insert_binheap_fifo(prog, "fifo")
    qc.insert_main(prog, fifo, num_cmds, keepgoing=keepgoing)
    return prog.program


if __name__ == "__main__":
    build().emit()
