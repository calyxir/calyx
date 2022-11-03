from calyx.builder import ProgramBuilder, const
from calyx import py_ast as ast


def build():
    prog = ProgramBuilder()
    main = prog.component("main")

    lhs = main.reg("lhs", 32)
    rhs = main.reg("rhs", 32)
    sum = main.reg("sum", 32)
    add = main.add("add", 32)

    update_operands = main.group("update_operands")
    update_operands.asgn(lhs.port("in"), const(32, 1))
    update_operands.asgn(rhs.port("in"), const(32, 41))
    update_operands.asgn(lhs.port("write_en"), const(1, 1))
    update_operands.asgn(rhs.port("write_en"), const(1, 1))
    update_operands.asgn(
        update_operands.done,
        const(1, 1),
        lhs.port("done") & rhs.port("done"),
    )

    compute_sum = main.group("compute_sum")
    compute_sum.asgn(add.port("left"), lhs.port("out"))
    compute_sum.asgn(add.port("right"), rhs.port("out"))
    compute_sum.asgn(sum.port("write_en"), const(1, 1))
    compute_sum.asgn(sum.port("in"), add.port("out"))
    compute_sum.asgn(compute_sum.done, sum.port("done"))

    main.control += [
        update_operands,
        compute_sum,
    ]

    return prog.program


if __name__ == '__main__':
    build().emit()
