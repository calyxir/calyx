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
    with update_operands:
        lhs["in"] = const(32, 1)
        rhs["in"] = const(32, 41)
        lhs.write_en = const(1, 1)
        rhs.write_en = const(1, 1)
        update_operands.done[lhs.port("done") & rhs.port("done")] = const(1, 1)

    compute_sum = main.group("compute_sum")
    with compute_sum:
        add.left = lhs.out
        add.right = rhs.out
        sum.write_en = const(1, 1)
        sum["in"] = add.out
        compute_sum.done = sum.done

    main.control += [
        update_operands,
        compute_sum,
    ]

    return prog.program


if __name__ == '__main__':
    build().emit()
