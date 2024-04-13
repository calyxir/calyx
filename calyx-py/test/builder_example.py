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


def insert_mux_component(prog):
    """Insert a multiplexer component into the program.
    The user provides two values and a select signal.
    If the select signal is high, the component outputs the sum of the two values.
    If the select signal is low, the component outputs the difference of the two values.
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

    # ANCHOR: add_and_store
    sum_group, _ = comp.add_store_in_reg(val1, val2, mux)
    # ANCHOR_END: adder_group_and_reg
    diff_group, _ = comp.sub_store_in_reg(val1, val2, mux)

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
    insert_mux_component(prog)
    return prog.program
    # ANCHOR_END: build


# ANCHOR: emit
if __name__ == "__main__":
    build().emit()
# ANCHOR_END: emit
