# pylint: disable=import-error
import calyx.builder as cb
import calyx.tuple as tup


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


def insert_binheap(prog, name):
    """Inserts the component `binheap` into the program.

    It is a minimum binary heap, represented as an array.

    The heap just supports the `push` operation.
    Its only inputs are `value` and `rank`, the value and rank to push to the queue.
    """
    comp: cb.ComponentBuilder = prog.component(name)
    # The rank and value to push to the heap
    rank = comp.input("rank", 64)
    value = comp.input("value", 64)

    swap = comp.cell("swap", insert_swap(prog, "swap", 128, 15, 4))
    tuplify = comp.cell("tuplify", tup.insert_tuplify(prog, "tuplify", 64, 64))
    untuplify = comp.cell("untuplify", tup.insert_untuplify(prog, "untuplify", 64, 64))
    

    mem = comp.comb_mem_d1("mem", 128, 15, 4, is_ref=True)
    # The memory to store the heap, represented as an array.
    # For now it has a hardcoded max length of 15, i.e., a binary heap of height 4.
    # Each cell of the memory is 128 bits wide: 64 for both rank and value.

    sub = comp.sub(4)
    rsh = comp.rsh(4)

    size = comp.reg(4)  # Current size
    parent_idx = comp.reg(4)
    parent_rankval = comp.reg(128)
    child_idx = comp.reg(4)
    child_rankval = comp.reg(128)

    read_parent = comp.mem_load_d1(mem, parent_idx.out, parent_rankval, "read_parent")
    read_child = comp.mem_load_d1(mem, child_idx.out, child_rankval, "read_child")

    with comp.group("find_parent_idx") as find_parent_idx:
        # Find the parent of the `child_idx`th element and store it in `parent_idx`.
        # parent_idx := floor((child_idx − 1) / 2)
        sub.left = child_idx.out
        sub.right = 1
        rsh.left = sub.out
        rsh.right = cb.const(4, 1)
        parent_idx.in_ = rsh.out
        parent_idx.write_en = cb.HI
        find_parent_idx.done = parent_idx.done

    # child_idx := size
    set_child_idx = comp.reg_store(child_idx, size.out)  
    # mem[child_idx] := (rank, value)
    with comp.group("store_new_rankval") as store_new_rankval:
        tuplify.fst = rank
        tuplify.snd = value

        mem.addr0 = child_idx.out
        mem.write_en = cb.HI
        mem.write_data = tuplify.tup 
        store_new_rankval.done = mem.done

    incr_size = comp.incr(size)
    
    child_rank = comp.reg(64)
    parent_rank = comp.reg(64)

    with comp.group("extract_child_rank") as extract_child_rank:
        untuplify.tup = child_rankval.out
        child_rank.write_en = cb.HI
        child_rank.in_ = untuplify.fst
        extract_child_rank.done = child_rank.done

    with comp.group("extract_parent_rank") as extract_parent_rank:
        untuplify.tup = parent_rankval.out
        parent_rank.write_en = cb.HI
        parent_rank.in_ = untuplify.fst
        extract_parent_rank.done = parent_rank.done

    child_lt_parent = comp.lt_use(child_rank.out, parent_rank.out)

    bubble_child_idx = comp.reg_store(child_idx, parent_idx.out, "bubble_child_idx")

    comp.control += [
        set_child_idx,
        store_new_rankval,
        incr_size,
        find_parent_idx,
        read_parent,
        extract_parent_rank,
        read_child,
        extract_child_rank,
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
                extract_parent_rank,
                read_child,
                extract_child_rank,
            ],
        ),
    ]

    return comp

def insert_split(prog, name):
    comp = prog.component(name)
    
    untuplify = comp.cell("untup", tup.insert_untuplify(prog, "untup", 64, 64))

    pairs = comp.comb_mem_d1("pairs", 128, 15, 4, is_ref=True)
    ranks = comp.comb_mem_d1("ranks", 64, 15, 4, is_ref=True)
    values = comp.comb_mem_d1("values", 64, 15, 4, is_ref=True)

    i = comp.reg(4)
    rank = comp.reg(64)
    value = comp.reg(64)
    
    cond_i = comp.lt_use(i.out, 15)

    with comp.group("read_pair") as read_pair:
        pairs.addr0 = i.out
        untuplify.tup = pairs.read_data

        rank.write_en = cb.HI
        rank.in_ = untuplify.fst

        value.write_en = cb.HI
        value.in_ = untuplify.snd

        read_pair.done = value.done @ rank.done


    store_rank = comp.mem_store_d1(ranks, i.out, rank.out, "store_rank")  
    store_value = comp.mem_store_d1(values, i.out, value.out, "store_value")  

    comp.control += [
            cb.while_with(cond_i, [read_pair, store_rank, store_value, comp.incr(i)]),
            read_pair,
            store_rank,
            store_value
        ]

    return comp


def insert_main(prog, binheap, split):
    """Inserts the main component into the program.
    Invokes the `binheap` component with 64-bit values 4 and 5,
    and a 64-bit memory of length 15.
    """
    comp = prog.component("main")
    binheap = comp.cell("binheap", binheap)
    split = comp.cell("split", split)

    mem = comp.comb_mem_d1("mem", 128, 15, 4, is_external=True)
    ranks = comp.comb_mem_d1("ranks", 64, 15, 4, is_external=True)
    values = comp.comb_mem_d1("values", 64, 15, 4, is_external=True)

    comp.control += [
        cb.invoke(binheap, in_value=cb.const(64, 9), in_rank=cb.const(64, 9), ref_mem=mem),
        cb.invoke(binheap, in_value=cb.const(64, 12), in_rank=cb.const(64, 12), ref_mem=mem),
        cb.invoke(binheap, in_value=cb.const(64, 6), in_rank=cb.const(64, 6), ref_mem=mem),
        cb.invoke(binheap, in_value=cb.const(64, 3), in_rank=cb.const(64, 3), ref_mem=mem),
        cb.invoke(split, ref_pairs=mem, ref_ranks=ranks, ref_values=values)
    ]

    return comp


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    binheap = insert_binheap(prog, "binheap")
    split = insert_split(prog, "split")
    _ = insert_main(prog, binheap, split)
    return prog.program


if __name__ == "__main__":
    build().emit()
