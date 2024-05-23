# pylint: disable=import-error
import calyx.builder as cb


def insert_cmp(prog, name, width):
    """Inserts the combinational component `cmp` into the program.

    It takes two `width`-bit inputs `a` and `b` and produces a 1-bit output `lt`.
    The output `lt` is set to 1 if `a` is less than `b`, and 0 otherwise.
    """

    comp = prog.comb_component(name)
    a = comp.input("a", width)
    b = comp.input("b", width)
    comp.output("lt", 1)

    lt_cell = comp.lt(width)

    with comp.continuous:
        lt_cell.left = a
        lt_cell.right = b
        comp.this().lt = lt_cell.out

    return comp


def insert_swap(prog, name, width, len, idx_w):
    """Inserts the component `swap` into the program.

    It takes two `idx_w`-bit inputs `a` and `b` and accepts a memory by reference.
    The memory is a `len`-element memory of `width`-bit elements.
    It swaps the values in the memory at addresses `a` and `b`.
    """

    comp = prog.component(name)
    a = comp.input("a", idx_w)
    b = comp.input("b", idx_w)
    mem = comp.comb_mem_d1("mem", width, len, idx_w, is_ref=True)

    mem_a = comp.reg(width)
    mem_b = comp.reg(width)
    temp_val = comp.reg(width)

    read_a = comp.mem_load_comb_mem_d1(mem, a, mem_a, "read_a")  # mem_a := mem[a]
    read_b = comp.mem_load_comb_mem_d1(mem, b, mem_b, "read_b")  # mem_b := mem[b]

    write_a = comp.mem_store_comb_mem_d1(
        mem, a, mem_a.out, "write_a"
    )  # mem[a] := mem_a
    write_b = comp.mem_store_comb_mem_d1(
        mem, b, mem_b.out, "write_b"
    )  # mem[b] := mem_b

    with comp.group("swap_registers") as swap_registers:
        # Swap the values at registers `a_val` and `b_val`
        temp_val.in_ = mem_a.out
        temp_val.write_en = cb.HI
        mem_a.in_ = mem_b.out
        mem_a.write_en = temp_val.done
        mem_b.in_ = temp_val.out
        mem_b.write_en = mem_a.done
        swap_registers.done = mem_b.done

    comp.control += [
        read_a,
        read_b,
        swap_registers,
        write_a,
        write_b,
    ]

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
    cmd = comp.input("cmd", 2)
    # If this is 0, we pop. If it is 1, we peek.
    # If it is 2, we push `value` to the queue.
    value = comp.input("value", 64)  # The value to push to the queue

    ans = comp.reg(64, "ans", is_ref=True)
    # If the user wants to pop, we will write the popped value to `ans`.
    err = comp.reg(1, "err", is_ref=True)
    # We'll raise this as a general error flag for overflow and underflow.

    swap = comp.cell("swap", insert_swap(prog, "swap", 64, 3, 4))
    cmp = comp.cell("cmp", insert_cmp(prog, "cmp", 64))

    mem = comp.comb_mem_d1("mem", 64, 3, 4, is_ref=True)
    # The memory to store the heap, represented as an array.
    # For now it has a hardcoded max length of 3, i.e., a binary heap of height 2.
    # Each cell of the memory is 64 bits wide.

    size = comp.reg(4)  # Active size, and also the next address to write to.

    add = comp.add(4)
    sub = comp.sub(4)
    lsh = comp.lsh(4)
    rsh = comp.rsh(4)

    parent_idx = comp.reg(4)
    parent_val = comp.reg(64)
    child_idx = comp.reg(4)
    child_val = comp.reg(64)

    load_parent = comp.mem_load_comb_mem_d1(
        mem, parent_idx.out, parent_val, "load_parent"
    )
    load_child = comp.mem_load_comb_mem_d1(mem, child_idx.out, child_val, "load_child")

    with comp.group("find_parent_idx") as find_parent_idx:
        # Find the parent of the `child`th element and store it in `parent`.
        # That is, parent := floor((child − 1) / 2)
        sub.left = child_idx.out
        sub.right = 1
        rsh.left = sub.out
        rsh.right = cb.const(4, 1)
        parent_idx.in_ = rsh.out
        parent_idx.write_en = cb.HI
        find_parent_idx.done = parent_idx.done

    # with comp.group("find_left_child") as find_child:
    #     # Find the left child of the `parent`th element and store it in `child`.
    #     # That is, child := 2*parent + 1
    #     mul.left = parent.out
    #     mul.right = 2
    #     mul.go = cb.HI
    #     add.left = mul.out
    #     add.right = 1
    #     child.in_ = add.out
    #     child.write_en = cb.HI
    #     find_child.done = child.done

    set_child_idx = comp.reg_store(child_idx, size.out)
    put_new_val_in_mem = comp.mem_store_comb_mem_d1(
        mem, child_idx.out, value, "put_new_val_in_mem"
    )

    incr_size = comp.incr(size)
    child_lt_parent = comp.lt_use(child_val.out, parent_val.out)
    bubble_child_idx = comp.reg_store(child_idx, parent_idx.out, "bubble_child_idx")

    comp.control += [
        set_child_idx,
        put_new_val_in_mem,
        incr_size,
        find_parent_idx,
        load_parent,
        load_child,
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
                load_parent,
                load_child,
            ],
        ),
    ]

    return comp


def insert_main(prog, binheap):
    """Inserts the main component into the program.
    Invokes the `binheap` component with 64-bit values 4 and 5,
    and a 64-bit memory of length 3.
    """
    comp = prog.component("main")
    binheap = comp.cell("binheap", binheap)

    mem = comp.comb_mem_d1("mem", 64, 3, 4, is_external=True)
    ans = comp.reg(64)
    err = comp.reg(1)

    comp.control += [
        cb.invoke(
            binheap,
            in_value=cb.const(64, 9),
            ref_mem=mem,
            ref_ans=ans,
            ref_err=err,
        ),
        cb.invoke(
            binheap,
            in_value=cb.const(64, 6),
            ref_mem=mem,
            ref_ans=ans,
            ref_err=err,
        ),
        # cb.invoke(
        #     binheap,
        #     in_value=cb.const(64, 3),
        #     ref_mem=mem,
        #     ref_ans=ans,
        #     ref_err=err,
        # ),
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
