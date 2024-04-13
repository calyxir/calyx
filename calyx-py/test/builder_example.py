import calyx.builder as cb


def add_adder_component(prog):
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


# ANCHOR: build
def build():
    prog = cb.Builder()
    add_adder_component(prog)
    return prog.program
    # ANCHOR_END: build


# ANCHOR: emit
if __name__ == "__main__":
    build().emit()
# ANCHOR_END: emit
