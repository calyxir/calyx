from calyx.builder import Builder
from calyx import py_ast as ast


def build():
    prog = Builder()
    main = prog.component("main")
    main.input("in", 32)
    main.output("out", 32)

    lhs = main.reg("lhs", 32)
    rhs = main.reg("rhs", 32)
    sum = main.reg("sum", 32)
    add = main.add("add", 32)

    # Bare name of the cell
    add_out = ast.CompPort(ast.CompVar("add"), "out")

    with main.group("update_operands") as update_operands:
        # Directly index cell ports using dot notation
        lhs.write_en = 1
        rhs.write_en = 1
        # `in` is a reserved keyword, so we use `in_` instead
        lhs.in_ = 1
        rhs.in_ = 41
        # Guards are specified using the `@` syntax
        update_operands.done = (lhs.done & rhs.done) @ 1

    with main.group("compute_sum") as compute_sum:
        add.left = lhs.out
        add.right = rhs.out
        sum.write_en = 1
        # Directly use the ast.CompPort object `add_out`.
        # This is useful HACK when we haven't defined the cell yet but still want
        # to use its ports.
        sum.in_ = add_out
        compute_sum.done = sum.done

    this = main.this()
    with main.continuous:
        this.in_ = this.out

    main.control += [
        update_operands,
        compute_sum,
    ]

    return prog.program


if __name__ == "__main__":
    build().emit()
