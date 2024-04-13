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
    sum = comp.reg("sum", 32)
    add = comp.add(32, "add")
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
        sum.in_ = add.out
        compute_sum.done = sum.done

    # ANCHOR: this_continuous
    # Use `this()` method to access the ports on the current component
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

    diff = comp.reg("diff", 32)
    ge = comp.ge(32, "ge")
    ge_reg = comp.reg("ge_reg", 1)

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

    with comp.continuous:
        comp.this().out = diff.out

    comp.control += [
        val1_ge_val2,
        cb.if_(ge_reg.out, diff_group_1, diff_group_2),
    ]

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
    mux = comp.reg("mux", 32)

    # ANCHOR: eq_use
    sel_eq_0 = comp.eq_use(sel, 0)
    # ANCHOR_END: eq_use

    # ANCHOR: sum_group_oneliner
    sum_group, _ = comp.add_store_in_reg(val1, val2, mux)
    # ANCHOR_END: sum_group_oneliner

    # ANCHOR: multi-component
    abs_diff = comp.cell("abs_diff", diff_comp)
    with comp.group("compute_diff") as diff_group:
        # We will use the `diff_comp` component.
        abs_diff.val1 = val1
        abs_diff.val2 = val2
        abs_diff.go = cb.HI
        mux.write_en = abs_diff.done
        mux.in_ = abs_diff.out
        diff_group.done = mux.done
    # ANCHOR_END: multi-component

    with comp.continuous:
        comp.this().out = mux.out

    # ANCHOR: if_with
    comp.control += cb.if_with(sel_eq_0, sum_group, diff_group)
    # ANCHOR_END: if_with

    return comp


# ANCHOR: build
def build():
    prog = cb.Builder()
    insert_adder_component(prog)
    diff_comp = insert_abs_diff_component(prog)
    insert_mux_component(prog, diff_comp)
    return prog.program
    # ANCHOR_END: build


# ANCHOR: emit
if __name__ == "__main__":
    build().emit()
# ANCHOR_END: emit
