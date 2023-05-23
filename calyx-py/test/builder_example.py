from calyx.builder import Builder, const
from calyx import py_ast as ast


def build():
    # ANCHOR: init
    prog = Builder()
    main = prog.component("main")
    main.input("in", 32)
    main.output("out", 32)
    # ANCHOR_END: init

    # ANCHOR: cells
    lhs = main.reg("lhs", 32)
    rhs = main.reg("rhs", 32)
    sum = main.reg("sum", 32)
    add = main.add("add", 32)
    # ANCHOR_END: cells

    # ANCHOR: bare
    # Bare name of the cell
    add_out = ast.CompPort(ast.CompVar("add"), "out")
    # ANCHOR_END: bare

    # ANCHOR: group_def
    with main.group("update_operands") as update_operands:
        # ANCHOR_END: group_def
        # ANCHOR: assigns
        # Directly index cell ports using dot notation
        lhs.write_en = 1
        # Builder attempts to infer the bitwidth of the constant
        rhs.write_en = 1
        # `in` is a reserved keyword, so we use `in_` instead
        lhs.in_ = 1
        # ANCHOR_END: assigns
        # ANCHOR: const
        # Explicilty sized constants when bitwidth inference may not work
        rhs.in_ = const(32, 41)
        # ANCHOR_END: const
        # ANCHOR: done
        # Guards are specified using the `@` syntax
        update_operands.done = (lhs.done & rhs.done) @ 1
        # ANCHOR_END: done

    with main.group("compute_sum") as compute_sum:
        add.left = lhs.out
        add.right = rhs.out
        sum.write_en = 1
        # Directly use the ast.CompPort object `add_out`.
        # This is useful HACK when we haven't defined the cell yet but still want
        # to use its ports.
        # ANCHOR: bare_use
        sum.in_ = add_out
        # ANCHOR_END: bare_use
        compute_sum.done = sum.done

    # ANCHOR: this
    # Use `this()` method to access the ports on the current component
    this = main.this()
    # ANCHOR_END: this
    # ANCHOR: continuous
    with main.continuous:
        this.in_ = this.out
    # ANCHOR_END: continuous

    # ANCHOR: control
    main.control += [
        update_operands,
        compute_sum,
    ]
    # ANCHOR_END: control

    # ANCHOR: return
    return prog.program
    # ANCHOR_END: return


# ANCHOR: emit
if __name__ == "__main__":
    build().emit()
# ANCHOR_END: emit
