# pylint: disable=import-error
import sys
import calyx.builder as cb
import calyx.queue_call as qc
from binheap import insert_binheap

FACTOR = 4


def insert_stable_binheap(prog, name, queue_len_factor=FACTOR):
    """Inserts the component `stable_binheap` into the program.

    It is a minimum binary heap that breaks ties via insertion order. That is, if 
    two elements with the same rank are inserted, the element that was inserted 
    first is the one that gets popped first.

    It has:
    - three inputs, `cmd`, `value`, and `rank`.
        - `cmd` has width 2.
        - `rank` has width 32.
        - `value` has width 32.
    - one memory, `mem`, of size `2**queue_size_factor`.
    - two ref registers, `ans` and `err`.
        - `ans` has width 32.
        - `err` has width 1.

    We use `below`, a binary heap that accepts 64-bit ranks and 32-bit values, and counter `i`.
    - To push a pair `(r, v)`, we push `(r << 32 + i, v)` to `below` and increment `i`.
    - To peak, we peak `below`.
    - To pop, we pop `below`.
    
    If we push `(r, v)` and then later `(r, v')`, we know `v` will be popped before `v'` 
    since it is pushed with higher rank to `below`.
    """

    comp = prog.component(name)

    below = comp.cell("below", insert_binheap(prog, "below", queue_len_factor, 64, 32))

    cmd = comp.input("cmd", 2)

    rank = comp.input("rank", 32)
    value = comp.input("value", 32)

    ans = comp.reg(32, "ans", is_ref=True)

    err = comp.reg(1, "err", is_ref=True)

    i = comp.reg(32)

    cat = comp.cat(32, 32)

    with comp.continuous:
        cat.left = rank
        cat.right = i.out

    comp.control += [
        cb.invoke(
            below, 
            in_value=value, 
            in_rank=cat.out, 
            in_cmd=cmd, 
            ref_ans=ans, 
            ref_err=err
        ),
        comp.incr(i)
    ]

    return comp


def build():
    """Top-level function to build the program."""
    num_cmds = int(sys.argv[1])
    keepgoing = "--keepgoing" in sys.argv
    prog = cb.Builder()
    binheap = insert_stable_binheap(prog, "binheap")
    qc.insert_main(prog, binheap, num_cmds, keepgoing=keepgoing, use_ranks=True)
    return prog.program


if __name__ == "__main__":
    build().emit()
