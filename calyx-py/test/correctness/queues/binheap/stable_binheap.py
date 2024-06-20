# pylint: disable=import-error
import calyx.builder as cb
from binheap import insert_binheap


def insert_stable_binheap(prog, name, queue_len_factor):
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


def insert_main(prog):
    """Inserts the `main` component into the program.

    Invokes the `binheap` component with the following workload:
        push(9, 9),
        push(12, 12),
        push(6, 6),
        push(3, 3),
        pop(),
        peek(),
        push(8, 8),
        push(10, 10),
        pop(),
        pop(),
        peek(),
        pop(),
        pop(),
        pop(),
        push(3, 2),
        push(4, 2),
        push(5, 2),
        push(6, 1),
        push(10, 1),
        pop(),
        pop(),
        pop(),
        pop(),
        pop()
    """

    comp = prog.component("main")

    queue_len_factor = 4

    binheap = insert_stable_binheap(prog, "stable_binheap", queue_len_factor)
    binheap = comp.cell("insertion_order_binheap", binheap)

    out = comp.seq_mem_d1("out", 32, 15, queue_len_factor, is_external=True)

    ans = comp.reg(32)
    err = comp.reg(1)

    index = 0

    def push(value, rank):
        return cb.invoke(
            binheap,
            in_value=cb.const(32, value),
            in_rank=cb.const(32, rank),
            in_cmd=cb.const(2, 2),
            ref_ans=ans,
            ref_err=err,
        )

    def pop_and_store():
        nonlocal index
        index += 1

        return [
            cb.invoke(
                binheap,
                in_value=cb.const(32, 50),
                in_rank=cb.const(32, 50),
                in_cmd=cb.const(2, 0),
                ref_ans=ans,
                ref_err=err,
            ),
            comp.mem_store_d1(out, index - 1, ans.out, f"store_ans_{index}"),
        ]

    def peek_and_store():
        nonlocal index
        index += 1

        return [
            cb.invoke(
                binheap,
                in_value=cb.const(32, 50),
                in_rank=cb.const(32, 50),
                in_cmd=cb.const(2, 1),
                ref_ans=ans,
                ref_err=err,
            ),
            comp.mem_store_d1(out, index - 1, ans.out, f"store_ans_{index}"),
        ]

    comp.control += [
        # works like the usual heap?
        push(9, 9),
        push(12, 12),
        push(6, 6),
        push(3, 3),
        pop_and_store(),
        peek_and_store(),
        push(8, 8),
        push(10, 10),
        pop_and_store(),
        pop_and_store(),
        peek_and_store(),
        pop_and_store(),
        pop_and_store(),
        pop_and_store(),
        # breaks ties correctly?
        push(3, 2),
        push(4, 2),
        push(5, 2),
        push(6, 1),
        push(10, 1),
        pop_and_store(),
        pop_and_store(),
        pop_and_store(),
        pop_and_store(),
        pop_and_store(),
    ]


def build():
    """Top-level function to build the program."""

    prog = cb.Builder()
    insert_main(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
