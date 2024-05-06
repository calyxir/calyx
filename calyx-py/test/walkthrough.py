import calyx.builder as cb


def insert_adder_component(prog):
    """Insert an adder component into the program.
    This is a contrived example:
    the component basically just wraps the adder primitive.
    """
    # ANCHOR: component
    comp = prog.component("adder")
    # ANCHOR_END: component

    # ANCHOR: ports
    val1 = comp.input("val1", 32)
    val2 = comp.input("val2", 32)
    comp.output("out", 32)
    # ANCHOR_END: ports

    # ANCHOR: cells
    sum = comp.reg(32)
    add = comp.add(32)
    # ANCHOR_END: cells

    # ANCHOR: group_def
    with comp.group("compute_sum") as compute_sum:
        # ANCHOR_END: group_def
        # ANCHOR: dot_notation
        add.left = val1
        add.right = val2
        # ANCHOR_END: dot_notation
        # ANCHOR: high_signal
        sum.write_en = cb.HI
        # ANCHOR_END: high_signal
        # `in` is a reserved keyword, so we use `in_` instead
        # ANCHOR: in_
        sum.in_ = add.out
        # ANCHOR_END: in_
        # ANCHOR: done
        compute_sum.done = sum.done
        # ANCHOR_END: done

    # Use `this()` method to access the ports on the current component
    # ANCHOR: this_continuous
    with comp.continuous:
        comp.this().out = sum.out
    # ANCHOR_END: this_continuous

    # ANCHOR: control
    comp.control += compute_sum
    # ANCHOR_END: control

    # ANCHOR: return
    return comp
    # ANCHOR_END: return


def insert_abs_diff_component(prog):
    """Insert an absolute difference component into the program.
    It takes two values and outputs the absolute difference between them.
    """
    comp = prog.component("abs_diff")

    val1 = comp.input("val1", 32)
    val2 = comp.input("val2", 32)
    comp.output("out", 32)

    diff = comp.reg(32)
    ge = comp.ge(32, "ge")
    ge_reg = comp.reg(1)

    # ANCHOR: sub_and_store
    diff_group_1, _ = comp.sub_store_in_reg(val1, val2, diff)
    # ANCHOR_END: sub_and_store
    diff_group_2, _ = comp.sub_store_in_reg(val2, val1, diff)

    with comp.group("val1_ge_val2") as val1_ge_val2:
        ge.left = val1
        ge.right = val2
        ge_reg.write_en = cb.HI
        ge_reg.in_ = ge.out
        val1_ge_val2.done = ge_reg.done

    # ANCHOR: lt_use_oneliner
    val2_lt_val1 = comp.lt_use(val2, val1)
    # ANCHOR_END: lt_use_oneliner

    with comp.continuous:
        comp.this().out = diff.out

    # ANCHOR: par_if_ifwith
    comp.control += cb.par(
        [
            val1_ge_val2,
            cb.if_(ge_reg.out, diff_group_1, diff_group_2),
        ],
        cb.if_with(val2_lt_val1, diff_group_2, diff_group_1),
    )
    # ANCHOR_END: par_if_ifwith
    # This is contrived; either of the `par` branches would have sufficed.

    return comp


def insert_mux_component(prog, diff_comp):
    """Insert a multiplexer component into the program.
    The user provides two values and a select signal.
    If the select signal is high, the component outputs the sum of the two values.
    If the select signal is low, the component outputs the
    absolute difference of the two values.
    """
    comp = prog.component("mux")

    val1 = comp.input("val1", 32)
    val2 = comp.input("val2", 32)
    sel = comp.input("sel", 1)
    comp.output("out", 32)
    mux = comp.reg(32)

    sel_eq_0 = comp.eq_use(sel, 0)
    sum_group, _ = comp.add_store_in_reg(val1, val2, mux)

    # We will use the `diff_comp` component to compute the absolute difference.
    # ANCHOR: multi-component
    abs_diff = comp.cell("abs_diff", diff_comp)
    with comp.group("compute_diff") as diff_group:
        abs_diff.val1 = val1
        abs_diff.val2 = val2
        abs_diff.go = cb.HI
        mux.write_en = abs_diff.done
        mux.in_ = abs_diff.out
        diff_group.done = mux.done
    # ANCHOR_END: multi-component

    with comp.continuous:
        comp.this().out = mux.out

    comp.control += cb.if_with(sel_eq_0, sum_group, diff_group)

    return comp


def insert_map_component(prog):
    """Insert a map component into the program.
    The user provides a 1-d memory of length 10, by reference.
    The user also provides a single value, `v`.
    We add `v` to each element in the memory.
    """
    comp = prog.component("map")
    # ANCHOR: comb_mem_d1_ref
    mem = comp.comb_mem_d1("mem", 32, 10, 32, is_ref=True)
    # ANCHOR_END: comb_mem_d1_ref
    v = comp.input("v", 32)

    i = comp.reg(8)
    # ANCHOR: incr_oneliner
    incr_i = comp.incr(i)
    # ANCHOR_END: incr_oneliner
    add = comp.add(32)

    # ANCHOR: width_inf
    i_lt_10 = comp.lt_use(i.out, 10)
    # ANCHOR_END: width_inf

    # ANCHOR: add_at_position_i
    with comp.group("add_at_position_i") as add_at_position_i:
        mem.addr0 = i.out
        add.left = mem.read_data
        add.right = v
        mem.write_en = add.done @ cb.HI
        mem.write_data = add.out
        add_at_position_i.done = mem.done
    # ANCHOR_END: add_at_position_i

    # ANCHOR: while_with
    comp.control += cb.while_with(i_lt_10, [add_at_position_i, incr_i])
    # ANCHOR_END: while_with

    return comp


def insert_main_component(prog, map):
    """Insert the main component into the program.
    This component will invoke the `adder`, `abs_diff`, `mux`, and `map` components.
    """

    comp = prog.component("main")
    map = comp.cell("map", map)
    # ANCHOR: ext_mem
    mymem = comp.comb_mem_d1("mymem", 32, 10, 32, is_external=True)
    # ANCHOR_END: ext_mem

    comp.control += [
        # ANCHOR: invoke
        cb.invoke(map, ref_mem=mymem, in_v=cb.const(32, 42))
        # ANCHOR_END: invoke
    ]


# ANCHOR: build
def build():
    prog = cb.Builder()
    insert_adder_component(prog)
    diff_comp = insert_abs_diff_component(prog)
    insert_mux_component(prog, diff_comp)
    map_comp = insert_map_component(prog)
    insert_main_component(prog, map_comp)
    return prog.program
    # ANCHOR_END: build


# ANCHOR: emit
if __name__ == "__main__":
    build().emit()
# ANCHOR_END: emit
