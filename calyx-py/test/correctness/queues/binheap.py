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
    mem = comp.comb_mem_d1("mem", width, len, idx_w, is_ref=True)

    val_a = comp.reg(width)
    val_b = comp.reg(width)

    load_a_a = comp.mem_load_d1(mem, a, val_a, "load_a")  # val_a := mem[a]
    load_b_b = comp.mem_load_d1(mem, b, val_b, "load_b")  # val_b := mem[b]

    store_a_b = comp.mem_store_d1(mem, a, val_b.out, "store_a")  # mem[a] := val_b
    store_b_a = comp.mem_store_d1(mem, b, val_a.out, "store_b")  # mem[b] := val_a

    comp.control += [load_a_a, load_b_b, store_a_b, store_b_a]

    return comp


def insert_binheap(prog, name, factor):
    """Inserts the component `binheap` into the program.

    It is a minimum binary heap, represented as an array.

    The heap just supports the `push` operation.
    Its only inputs are `value` and `rank`, the value and rank to push to the queue.
    """
    comp: cb.ComponentBuilder = prog.component(name)
    
    n = (2**factor) - 1

    cmd = comp.input("cmd", 2)
    # If it is 0, we pop.
    # If it is 1, we peek.
    # If it is 2, we push `(rank, value)` to the queue.

    # The rank and value to push to the heap.
    rank = comp.input("rank", 32)
    value = comp.input("value", 32)

    swap = comp.cell("swap", insert_swap(prog, "swap", 64, n, factor))
    tuplify = comp.cell("tuplify", insert_tuplify(prog, "tuplify", 32, 32))
    untuplify = comp.cell("untuplify", insert_untuplify(prog, "untuplify", 32, 32))

    mem = comp.comb_mem_d1("mem", 64, n, factor, is_ref=True)
    # The memory to store the heap, represented as an array.
    # Each cell of the memory is 64 bits wide: 32 for both rank and value.

    ans = comp.reg(32, "ans", is_ref=True)
    # If the user wants to pop or peek, we will write the value to `ans`.

    err = comp.reg(1, "err", is_ref=True)
    # We'll raise this as a general error flag for overflow and underflow.

    raise_err = comp.reg_store(err, 1, "raise_err")  # err := 1

    size = comp.reg(factor) # The active length of the heap.

    # Cells and groups to check which command we got.
    cmd_eq_0 = comp.eq_use(cmd, 0)
    cmd_eq_1 = comp.eq_use(cmd, 1)
    cmd_eq_2 = comp.eq_use(cmd, 2)
    cmd_lt_2 = comp.lt_use(cmd, 2)

    # Cells and groups to check for overflow and underflow.
    size_eq_0 = comp.eq_use(size.out, 0)
    size_eq_max = comp.eq_use(size.out, n)

    current_idx = comp.reg(factor)
    current_rankval = comp.reg(64)
    current_rank = comp.reg(32)

    parent_idx = comp.reg(factor)
    parent_rankval = comp.reg(64)
    parent_rank = comp.reg(32)

    child_1_idx = comp.reg(factor)
    child_1_rankval = comp.reg(factor)
    child_1_rank = comp.reg(32)

    child_2_idx = comp.reg(factor)
    child_2_rankval = comp.reg(factor)
    child_2_rank = comp.reg(32)

    read_parent = comp.mem_load_d1(mem, parent_idx.out, parent_rankval, "read_parent")
    read_current = comp.mem_load_d1(mem, current_idx.out, current_rankval, "read_current")
    read_child_1 = comp.mem_load_d1(mem, child_1_idx.out, child_1_rankval, "read_child_1")
    read_child_2 = comp.mem_load_d1(mem, child_2_idx.out, child_2_rankval, "read_child_2")

    sub = comp.sub(factor)
    rsh = comp.rsh(factor)
    add = comp.add(factor)
    lsh = comp.lsh(factor)

    with comp.group("find_parent_idx") as find_parent_idx:
        # Find the parent of the `current_idx`th element and store it in `parent_idx`.
        # parent_idx := floor((current_idx − 1) / 2)
        sub.left = current_idx.out
        sub.right = 1
        rsh.left = sub.out
        rsh.right = cb.const(factor, 1) # can this just be one?
        parent_idx.in_ = rsh.out
        parent_idx.write_en = cb.HI
        find_parent_idx.done = parent_idx.done

    with comp.group("find_child_idx") as find_child_idx:
        # Find the children of `current_idx`th element and store it in child_1_idx and child_2_idx.
        # child_1_idx := (2 * current_idx)  + 1
        # child_2_idx := (2 * current_idx)  + 2
        lsh.left = current_idx.out
        lsh.right = cb.const(factor, 1)
        add.left = 1
        add.right = lsh.out
        child_1_idx.write_en = cb.HI
        child_1_idx.in_ = add.out
        add.left = child_1_idx.done @ 1
        add.right = child_1_idx.done @ child_1_idx.out
        child_2_idx.write_en = child_1_idx.done @ cb.HI
        child_2_idx.out = child_1_idx.done @ add.out

    # current_idx := 0
    set_idx_zero = comp.reg_store(current_idx, 0)

    # mem[current_idx] := (rank, value)
    with comp.group("store_rankval") as store_rankval:
        tuplify.fst = rank
        tuplify.snd = value
        mem.addr0 = current_idx.out
        mem.write_en = cb.HI
        mem.write_data = tuplify.tup 
        store_rankval.done = mem.done

    # (output, _) := input
    def extract_fst(name, input, output):
        with comp.group(name) as extract_fst:
            untuplify.tup = input.out
            output.write_en = cb.HI
            output.in_ = untuplify.fst
            extract_fst.done = output.done

        return extract_fst

    extract_current_rank = extract_fst("extract_current_rank", 
                                       current_rankval, 
                                       current_rank)
    extract_parent_rank = extract_fst("extract_parent_rank", 
                                      parent_rankval, 
                                      parent_rank)
    extract_child_1_rank = extract_fst("extract_child_1_rank", 
                                       child_1_rankval, 
                                       child_1_rank)
    extract_child_2_rank = extract_fst("extract_child_2_rank", 
                                      child_2_rankval, 
                                      child_2_rank)
    
    # current_rank < parent_rank
    current_lt_parent = comp.lt_use(current_rank.out, parent_rank.out)
    
    lt_1 = comp.lt(factor)
    lt_2 = comp.lt(factor)
    lt_3 = comp.lt(factor)
    lt_4 = comp.lt(factor)
    and_1 = comp.and_(1)
    and_2 = comp.and_(1)
    or_ = comp.or_(1)

    # (child_1_idx < size && child_1_rank < current_rankval)
    # ||
    # (child_2_idx < size && child_2_rank < current_rankval)
    with comp.comb_group("current_gt_children") as current_gt_children:
        # child_1_idx < size 
        lt_1.left = child_1_idx.out
        lt_1.right = size.out

        # child_2_idx < size 
        lt_3.left = child_2_idx.out
        lt_3.right = size.out

        # child_1_rank < current_rankval
        lt_2.left = child_1_rank.out
        lt_2.right = current_rankval.out

        # child_2_rank < current_rankval
        lt_4.left = child_2_rank.out
        lt_4.right = current_rankval.out

        and_1.left = lt_1.out
        and_1.right = lt_2.out

        and_2.left = lt_3.out
        and_2.right = lt_4.out

        or_.left = and_1.out
        or_.right = and_2.out

    peak = comp.mem_load_d1(mem, 0, ans, "peak")

    pop = [
        peak,
        comp.decr(size),
        comp.reg_store(current_idx, size.out),
        read_current,
        comp.mem_store_d1(mem, 0, current_rankval.out, "store_current_rankval"),
        comp.reg_store(current_idx, 0),
        extract_current_rank,
        find_child_idx,
        read_child_1,
        extract_child_1_rank,
        read_child_2,
        extract_child_2_rank,
        cb.while_with(cb.CellAndGroup(or_, current_gt_children), 
                [
                    cb.if_with(cb.CellAndGroup(and_1, current_gt_children), 
                            [
                                cb.invoke(swap, in_a=child_1_idx.out, in_b=current_idx.out, ref_mem=mem), 
                                comp.reg_store(current_idx, child_1_idx.out, "set_idx_child_1")
                            ],
                            cb.if_with(cb.CellAndGroup(and_2, current_gt_children), 
                                    [
                                        cb.invoke(swap, in_a=child_2_idx.out, in_b=current_idx.out, ref_mem=mem), 
                                        comp.reg_store(current_idx, child_2_idx.out, "set_idx_child_2")
                                    ])
                           ),
                    find_child_idx,
                    read_child_1,
                    extract_child_1_rank,
                    read_child_2,
                    extract_child_2_rank
                ])
    ] 

    push = [
        comp.reg_store(current_idx, size.out),
        store_rankval,
        comp.incr(size),
        find_parent_idx,
        read_parent,
        extract_parent_rank,
        read_current,
        extract_current_rank,
        cb.while_with(current_lt_parent,
                [
                    cb.invoke(swap, in_a=parent_idx.out, in_b=current_idx.out, ref_mem=mem),
                    comp.reg_store(current_idx, parent_idx.out, "set_idx_parent"),
                    find_parent_idx,
                    read_parent,
                    extract_parent_rank
                ])
    ]

    raise_error = comp.reg_store(err, 1, "raise_err")  

    comp.control += [
            cb.if_with(cmd_eq_0, 
                cb.if_with(size_eq_0, raise_error, pop), 
                cb.if_with(cmd_eq_1, 
                    cb.if_with(size_eq_0, raise_error, peak),
                    cb.if_with(cmd_eq_2, 
                        cb.if_with(size_eq_max, raise_error, push), 
                        raise_error
                    )
                )
            )
    ]

    return comp

def insert_split(prog, name, factor):
    comp = prog.component(name)

    n = (2**factor) - 1
    
    untup = comp.cell("untup", insert_untuplify(prog, "untup", 32, 32))

    pairs = comp.comb_mem_d1("pairs", 64, n, factor, is_ref=True)
    ranks = comp.comb_mem_d1("ranks", 32, n, factor, is_ref=True)
    values = comp.comb_mem_d1("values", 32, n, factor, is_ref=True)

    i = comp.reg(factor)
    rank = comp.reg(32)
    value = comp.reg(32)
    
    cond_i = comp.lt_use(i.out, n)

    with comp.group("read_pair") as read_pair:
        pairs.addr0 = i.out
        untup.tup = pairs.read_data

        rank.write_en = cb.HI
        rank.in_ = untup.fst

        value.write_en = cb.HI
        value.in_ = untup.snd

        read_pair.done = value.done @ rank.done


    store_rank = comp.mem_store_d1(ranks, i.out, rank.out, "store_rank")  
    store_value = comp.mem_store_d1(values, i.out, value.out, "store_value")  

    comp.control += [cb.while_with(cond_i, 
                                   [ 
                                       read_pair, 
                                       store_rank, 
                                       store_value, 
                                       comp.incr(i)
                                   ])
                    ]

    return comp


def insert_main(prog, binheap, split):
    """Inserts the main component into the program.
    Invokes the `binheap` component with 32-bit values 4 and 5,
    and a 64-bit memory of length 15.
    """
    comp = prog.component("main")

    factor = 4

    binheap = insert_binheap(prog, "binheap", factor)
    split = insert_split(prog, "split", factor)

    binheap = comp.cell("binheap", binheap)
    split = comp.cell("split", split)

    mem = comp.comb_mem_d1("mem", 64, 15, factor, is_external=True)
    ranks = comp.comb_mem_d1("ranks", 32, 15, factor, is_external=True)
    values = comp.comb_mem_d1("values", 32, 15, factor, is_external=True)

    comp.control += [
        cb.invoke(binheap, in_value=cb.const(32, 9), in_rank=cb.const(32, 9), ref_mem=mem),
        cb.invoke(binheap, in_value=cb.const(32, 12), in_rank=cb.const(32, 12), ref_mem=mem),
        cb.invoke(binheap, in_value=cb.const(32, 6), in_rank=cb.const(32, 6), ref_mem=mem),
        cb.invoke(binheap, in_value=cb.const(32, 3), in_rank=cb.const(32, 3), ref_mem=mem),
        cb.invoke(split, ref_pairs=mem, ref_ranks=ranks, ref_values=values)
    ]

    return comp


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    binheap = insert_binheap(prog, "binheap", 4)
    split = insert_split(prog, "split", 4)
    _ = insert_main(prog, binheap, split)
    return prog.program


if __name__ == "__main__":
    build().emit()
