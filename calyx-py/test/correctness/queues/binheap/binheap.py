# pylint: disable=import-error
import calyx.builder as cb
from calyx.tuple import insert_tuplify, insert_untuplify


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


def insert_binheap(prog, name, queue_len_factor):
    """Inserts the component `binheap` into the program.

    It is a minimum binary heap, represented as an array.

    It has:
    - three inputs, `cmd`, `value`, and `rank`.
    - one memory, `mem`, of size `(2**queue_len_factor)`.
    - two ref registers, `ans` and `err`.
    """

    comp = prog.component(name)

    max_queue_len = 2**queue_len_factor

    cmd = comp.input("cmd", 2)
    # If it is 0, we pop.
    # If it is 1, we peek.
    # If it is 2, we push `(rank, value)` to the queue.

    # The value and associated rank to push to the heap.
    rank = comp.input("rank", 64)
    value = comp.input("value", 32)

    swap = comp.cell(
        "swap", insert_swap(prog, "swap", 96, max_queue_len, queue_len_factor)
    )
    tuplify = comp.cell("tuplify", insert_tuplify(prog, "tuplify", 64, 32))
    untuplify = comp.cell("untuplify", insert_untuplify(prog, "untuplify", 64, 32))

    mem = comp.seq_mem_d1("mem", 96, max_queue_len, queue_len_factor)
    # The memory to store the heap, represented as an array.
    # Each cell of the memory is 96 bits wide: a 64-bit rank and a 32-bit value.

    ans = comp.reg(32, "ans", is_ref=True)
    # If the user wants to pop or peek, we will write the value to `ans`.

    err = comp.reg(1, "err", is_ref=True)
    # We'll raise this as a general error flag for overflow and underflow.

    size = comp.reg(queue_len_factor)  # The active length of the heap.

    # Cells and groups to check which command we got.
    cmd_eq_0 = comp.eq_use(cmd, 0)
    cmd_eq_1 = comp.eq_use(cmd, 1)
    cmd_eq_2 = comp.eq_use(cmd, 2)

    # Cells and groups to check for overflow and underflow.
    size_eq_0 = comp.eq_use(size.out, 0)
    size_eq_max = comp.eq_use(size.out, max_queue_len - 1)

    current_idx = comp.reg(queue_len_factor)
    current_rank = comp.reg(64)

    parent_idx = comp.reg(queue_len_factor)
    parent_rank = comp.reg(64)

    child_l_idx = comp.reg(queue_len_factor)
    child_l_rank = comp.reg(64)

    child_r_idx = comp.reg(queue_len_factor)
    child_r_rank = comp.reg(64)

    # current_idx := 0
    set_idx_zero = comp.reg_store(current_idx, 0, "set_idx_zero")

    # current_idx := size
    set_idx_size = comp.reg_store(current_idx, size.out, "set_idx_size")

    # current_idx := child_l_idx
    set_idx_child_l = comp.reg_store(current_idx, child_l_idx.out, "set_idx_child_l")

    # current_idx := child_r_idx
    set_idx_child_r = comp.reg_store(current_idx, child_r_idx.out, "set_idx_child_r")

    # current_idx := parent_idx
    set_idx_parent = comp.reg_store(current_idx, parent_idx.out, "set_idx_parent")

    # err := 1
    raise_err = comp.reg_store(err, 1, "raise_err")

    # err := 0
    lower_err = comp.reg_store(err, 0, "lower_err")

    sub = comp.sub(queue_len_factor)
    rsh = comp.rsh(queue_len_factor)
    with comp.group("find_parent_idx") as find_parent_idx:
        # Find the parent of the `current_idx`th element and store it in `parent_idx`.
        # parent_idx := floor((current_idx − 1) / 2)
        sub.left = current_idx.out
        sub.right = 1
        rsh.left = sub.out
        rsh.right = cb.const(queue_len_factor, 1)
        parent_idx.in_ = rsh.out
        parent_idx.write_en = cb.HI
        find_parent_idx.done = parent_idx.done

    add_1 = comp.add(queue_len_factor)
    add_2 = comp.add(queue_len_factor)
    lsh = comp.lsh(queue_len_factor)
    with comp.group("find_child_idx") as find_child_idx:
        # Find the children of `current_idx`th element and store
        # them in child_l_idx and child_r_idx.
        # child_l_idx := (2 * current_idx) + 1
        # child_r_idx := (2 * current_idx) + 2
        lsh.left = current_idx.out
        lsh.right = cb.const(queue_len_factor, 1)
        add_1.left = 1
        add_1.right = lsh.out
        child_l_idx.write_en = cb.HI
        child_l_idx.in_ = add_1.out
        add_2.left = child_l_idx.done @ 1
        add_2.right = child_l_idx.done @ child_l_idx.out
        child_r_idx.write_en = child_l_idx.done @ cb.HI
        child_r_idx.in_ = child_l_idx.done @ add_2.out
        find_child_idx.done = child_r_idx.done

    # mem[current_idx] := (rank, value)
    with comp.group("store_rank_and_value") as store_rank_and_value:
        tuplify.fst = rank
        tuplify.snd = value
        mem.addr0 = current_idx.out
        mem.write_en = cb.HI
        mem.content_en = cb.HI
        mem.write_data = tuplify.tup
        store_rank_and_value.done = mem.done

    # (out, _) := mem[idx]
    def extract_fst(name, idx, out):
        with comp.group(name) as extract_fst:
            mem.addr0 = idx
            mem.content_en = cb.HI
            untuplify.tup = mem.done @ mem.read_data
            out.write_en = mem.done @ cb.HI
            out.in_ = mem.done @ untuplify.fst
            extract_fst.done = out.done

        return extract_fst

    # (_, out) := mem[indx]
    def extract_snd(name, idx, out):
        with comp.group(name) as extract_snd:
            mem.addr0 = idx
            mem.content_en = cb.HI
            untuplify.tup = mem.done @ mem.read_data
            out.write_en = mem.done @ cb.HI
            out.in_ = mem.done @ untuplify.snd
            extract_snd.done = out.done

        return extract_snd

    extract_current_rank = extract_fst(
        "extract_current_rank",
        current_idx.out,
        current_rank,
    )
    extract_parent_rank = extract_fst(
        "extract_parent_rank",
        parent_idx.out,
        parent_rank,
    )
    extract_child_l_rank = extract_fst(
        "extract_child_l_rank",
        child_l_idx.out,
        child_l_rank,
    )
    extract_child_r_rank = extract_fst(
        "extract_child_r_rank",
        child_r_idx.out,
        child_r_rank,
    )

    # current_rank < parent_rank
    current_lt_parent = comp.lt_use(current_rank.out, parent_rank.out)

    le_1 = comp.le(queue_len_factor)
    le_2 = comp.le(64)
    if_or = comp.or_(1)
    with comp.comb_group("child_l_swap") as child_l_swap:
        # Check if the `current_idx`th element should be swapped with its left child.
        # size <= child_r_idx OR child_l_rank <= child_r_rank
        le_1.left = size.out
        le_1.right = child_r_idx.out

        le_2.left = child_l_rank.out
        le_2.right = child_r_rank.out

        if_or.left = le_1.out
        if_or.right = le_2.out

    lt_1 = comp.lt(queue_len_factor)
    lt_2 = comp.lt(64)
    lt_3 = comp.lt(queue_len_factor)
    lt_4 = comp.lt(64)
    and_1 = comp.and_(1)
    and_2 = comp.and_(1)
    while_or = comp.or_(1)
    with comp.comb_group("current_gt_children") as current_gt_children:
        # Check if the `current_idx`th element should be swapped with its left OR right.
        # child_l_idx < size AND child_l_rank < current_rank
        # OR
        # child_r_idx < size AND child_r_rank < current_rank
        lt_1.left = child_l_idx.out
        lt_1.right = size.out

        lt_2.left = child_l_rank.out
        lt_2.right = current_rank.out

        lt_3.left = child_r_idx.out
        lt_3.right = size.out

        lt_4.left = child_r_rank.out
        lt_4.right = current_rank.out

        and_1.left = lt_1.out
        and_1.right = lt_2.out

        and_2.left = lt_3.out
        and_2.right = lt_4.out

        while_or.left = and_1.out
        while_or.right = and_2.out

    peek = extract_snd("peek", 0, ans)

    pop = [
        peek,
        comp.decr(size),
        set_idx_zero,
        cb.invoke(swap, in_a=current_idx.out, in_b=size.out, ref_mem=mem),
        extract_current_rank,
        find_child_idx,
        extract_child_l_rank,
        extract_child_r_rank,
        # Bubble Down
        cb.while_with(
            cb.CellAndGroup(while_or, current_gt_children),
            [
                cb.if_with(
                    cb.CellAndGroup(if_or, child_l_swap),
                    [
                        cb.invoke(
                            swap,
                            in_a=child_l_idx.out,
                            in_b=current_idx.out,
                            ref_mem=mem,
                        ),
                        set_idx_child_l,
                    ],
                    [
                        cb.invoke(
                            swap,
                            in_a=child_r_idx.out,
                            in_b=current_idx.out,
                            ref_mem=mem,
                        ),
                        set_idx_child_r,
                    ],
                ),
                find_child_idx,
                extract_child_l_rank,
                extract_child_r_rank,
            ],
        ),
    ]

    push = [
        set_idx_size,
        store_rank_and_value,
        comp.incr(size),
        find_parent_idx,
        extract_parent_rank,
        extract_current_rank,
        # Bubble Up
        cb.while_with(
            current_lt_parent,
            [
                cb.invoke(swap, in_a=parent_idx.out, in_b=current_idx.out, ref_mem=mem),
                set_idx_parent,
                find_parent_idx,
                extract_parent_rank,
            ],
        ),
    ]

    comp.control += [
        lower_err,
        cb.if_with(
            cmd_eq_0,
            cb.if_with(size_eq_0, raise_err, pop),
            cb.if_with(
                cmd_eq_1,
                cb.if_with(size_eq_0, raise_err, peek),
                cb.if_with(
                    cmd_eq_2, cb.if_with(size_eq_max, raise_err, push), raise_err
                ),
            ),
        ),
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
        push(3, 3),
        push(4, 4),
        push(5, 5),
        push(6, 6),
        push(10, 10),
        pop()
    """

    comp = prog.component("main")

    queue_len_factor = 4

    binheap = insert_binheap(prog, "binheap", queue_len_factor)
    binheap = comp.cell("binheap", binheap)

    out = comp.seq_mem_d1("out", 32, 15, queue_len_factor, is_external=True)

    ans = comp.reg(32)
    err = comp.reg(1)

    index = 0

    def push(value, rank):
        return cb.invoke(
            binheap,
            in_value=cb.const(32, value),
            in_rank=cb.const(64, rank),
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
                in_rank=cb.const(64, 50),
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
                in_rank=cb.const(64, 50),
                in_cmd=cb.const(2, 1),
                ref_ans=ans,
                ref_err=err,
            ),
            comp.mem_store_d1(out, index - 1, ans.out, f"store_ans_{index}"),
        ]

    comp.control += [
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
        push(3, 3),
        push(4, 4),
        push(5, 5),
        push(6, 6),
        push(10, 10),
        pop_and_store(),
    ]


def build():
    """Top-level function to build the program."""

    prog = cb.Builder()
    insert_main(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
