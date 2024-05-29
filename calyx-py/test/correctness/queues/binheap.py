# pylint: disable=import-error
import calyx.builder as cb


def insert_swap(prog, name, width, len, idx_w):
    """Inserts the component `swap` into the program.

    It takes two `idx_w`-bit inputs `a` and `b` and accepts a memory by reference.
    The memory is a `len`-element memory of `width`-bit elements.
    It swaps the values in the memory at addresses `a` and `b`.
    """

    comp = prog.component(name)
    a = comp.input("a", idx_w)
    b = comp.input("b", idx_w)
    mem = comp.seq_mem_d1("mem", width, len, idx_w, is_ref=True)

    val_a = comp.reg(width)
    val_b = comp.reg(width)

    load_a_a = comp.mem_load_d1(mem, a, val_a, "load_a")  # val_a := mem[a]
    load_b_b = comp.mem_load_d1(mem, b, val_b, "load_b")  # val_b := mem[b]

    store_a_b = comp.mem_store_d1(mem, a, val_b.out, "store_a")  # mem[a] := val_b
    store_b_a = comp.mem_store_d1(mem, b, val_a.out, "store_b")  # mem[b] := val_a

    comp.control += [load_a_a, load_b_b, store_a_b, store_b_a]

    return comp


def insert_binheap(prog, name):
    """Inserts the component `binheap` into the program.

    It is a minimum binary heap, represented as an array.

    It follows the interface of the `pifo` component:
    It has three inputs:
    - `cmd`: tells us what operation to execute.
    The heap supports the operations `pop`, `peek`, and `push`.
    - `value`: the value to push to the queue.

    If an answer is expected, it is written to the `ans` register.
    If an error occurs, the `err` register is set to 1.
    """
    comp: cb.ComponentBuilder = prog.component(name)
    value = comp.input("value", 64)  # The value to push to the queue

    swap = comp.cell("swap", insert_swap(prog, "swap", 64, 15, 4))

    mem = comp.seq_mem_d1("mem", 64, 15, 4, is_ref=True)
    # The memory to store the heap, represented as an array.
    # For now it has a hardcoded max length of 15, i.e., a binary heap of height 4.
    # Each cell of the memory is 64 bits wide.

    size = comp.reg(4)  # Active size

    sub = comp.sub(4)
    rsh = comp.rsh(4)
    eq = comp.eq(4)

    parent_idx = comp.reg(4)
    parent_val = comp.reg(64)
    child_idx = comp.reg(4)
    child_val = comp.reg(64)

    read_parent = comp.mem_load_d1(mem, parent_idx.out, parent_val, "read_parent")
    read_child = comp.mem_load_d1(mem, child_idx.out, child_val, "read_child")

    with comp.group("find_parent_idx") as find_parent_idx:
        # Find the parent of the `child`th element and store it in `parent`.
        # If child is 0, parent is 0.
        # Otherwise, parent := floor((child − 1) / 2)

        # Case when child = 0: parent := 0
        eq.left = child_idx.out
        eq.right = 0
        parent_idx.in_ = eq.out @ cb.const(4, 0)
        parent_idx.write_en = eq.out @ cb.HI

        # Else case: parent := floor((child − 1) / 2)
        sub.left = ~eq.out @ child_idx.out
        sub.right = ~eq.out @ 1
        rsh.left = ~eq.out @ sub.out
        rsh.right = ~eq.out @ cb.const(4, 1)
        parent_idx.in_ = ~eq.out @ rsh.out
        parent_idx.write_en = ~eq.out @ cb.HI

        # In either case, we are done when parent_idx is done.
        find_parent_idx.done = parent_idx.done

    set_child_idx = comp.reg_store(child_idx, size.out)  # child_idx := size
    store_new_val = comp.mem_store_d1(
        mem, child_idx.out, value, "store_new_val"
    )  # mem[child_idx] := value
    incr_size = comp.incr(size)
    child_lt_parent = comp.lt_use(child_val.out, parent_val.out)
    bubble_child_idx = comp.reg_store(child_idx, parent_idx.out, "bubble_child_idx")

    comp.control += [
        set_child_idx,
        store_new_val,
        incr_size,
        find_parent_idx,
        read_parent,
        read_child,
        cb.while_with(
            child_lt_parent,
            [
                cb.invoke(
                    swap,
                    in_a=parent_idx.out,
                    in_b=child_idx.out,
                    ref_mem=mem,
                ),
                bubble_child_idx,
                find_parent_idx,
                read_parent,
                read_child,
            ],
        ),
    ]

    return comp


def insert_main(prog, binheap):
    """Inserts the main component into the program.
    Invokes the `binheap` component with 64-bit values 4 and 5,
    and a 64-bit memory of length 15.
    """
    comp = prog.component("main")
    binheap = comp.cell("binheap", binheap)

    mem = comp.seq_mem_d1("mem", 64, 15, 4, is_external=True)

    comp.control += [
        cb.invoke(binheap, in_value=cb.const(64, 9), ref_mem=mem),
        cb.invoke(binheap, in_value=cb.const(64, 12), ref_mem=mem),
        cb.invoke(binheap, in_value=cb.const(64, 6), ref_mem=mem),
        cb.invoke(binheap, in_value=cb.const(64, 3), ref_mem=mem),
    ]

    return comp


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    binheap = insert_binheap(prog, "binheap")
    _ = insert_main(prog, binheap)
    return prog.program


if __name__ == "__main__":
    build().emit()
