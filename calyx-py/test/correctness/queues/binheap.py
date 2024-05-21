# pylint: disable=import-error
import calyx.builder as cb
import calyx.queue_call as qc


def insert_binheap(prog, name):
    """Inserts the component `binheap` into the program.

    It is a minimum binary heap, represented as an array.

    It follows the interface of the `pifo` component:
    It has inputs:
    - `cmd`: tells us what operation to execute.
    The heap supports the operations `pop`, `peek`, and `push`.
    - `value`: the value to push to the queue.
    - `rank`: the rank with which to push the value.

    If an answer is expected, it is written to the `ans` register.
    If an error occurs, the `err` register is set to 1.
    """
    binheap: cb.ComponentBuilder = prog.component(name)
    cmd = binheap.input("cmd", 2)
    # If this is 0, we pop. If it is 1, we peek.
    # If it is 2, we push `value` to the queue.
    value = binheap.input("value", 32)  # The value to push to the queue
    rank = binheap.input("rank", 32)  # The rank with which to push the value

    ans = binheap.reg(32, "ans", is_ref=True)
    # If the user wants to pop, we will write the popped value to `ans`.
    err = binheap.reg(1, "err", is_ref=True)
    # We'll raise this as a general error flag for overflow and underflow.

    mem = binheap.seq_mem_d1("mem", 64, 15, 4)
    # The memory to store the heap, represented as an array.
    # For now it has a hardcoded max length of 15, i.e., a binary heap of height 4.
    # The memory is 64 bits wide:
    # the first 32 bits represent the value,
    # and the next 32 bits represent the rank.

    next_write = binheap.reg(4)  # The next address to write to.
    i = binheap.reg(4)  # The index of the element we are currently looking at.
    j = binheap.reg(4)  # Another register work scrach work.

    add = binheap.add(4)
    sub = binheap.sub(64)
    mul = binheap.mult_pipe(64)
    div = binheap.div_pipe(64)

    with binheap.group("find_parent") as find_parent:
        # Find the parent of the `i`th element and store it in `j`.
        # That is, j := floor((i − 1) / 2)
        sub.left = i.out
        sub.right = 1
        sub.go = cb.HI
        div.left = sub.out
        div.right = 2
        div.go = sub.done
        j.in_ = div.out
        j.go = div.done
        find_parent.done = j.done

    with binheap.group("find_left_child") as find_left_child:
        # Find the left child of the `i`th element and store it in `j`.
        # That is, j := 2i + 1
        mul.left = i.out
        mul.right = 2
        mul.go = cb.HI
        add.left = mul.out
        add.right = 1
        add.go = mul.done
        j.in_ = add.out
        j.go = add.done
        find_left_child.done = j.done

    valuereg = binheap.reg(32)
    rankreg = binheap.reg(32)
    value_store = binheap.reg_store(valuereg, value)
    rank_store = binheap.reg_store(rankreg, rank)

    _, _ = binheap.tuplify(valuereg, rankreg)

    return binheap


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    _ = insert_binheap(prog, "binheap")
    return prog.program


if __name__ == "__main__":
    build().emit()
