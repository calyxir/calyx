from calyx.builder import Builder


def build():
    prog = Builder()
    main = prog.component("main")

    lhs = main.reg("lhs", 32)
    rhs = main.reg("rhs", 32)
    sum = main.reg("sum", 32)
    add = main.add("add", 32)

    with main.group("update_operands") as update_operands:
        lhs.in_ = 1
        rhs.in_ = 41
        lhs.write_en = 1
        rhs.write_en = 1
        update_operands.done = (lhs.done & rhs.done) @ 1

    with main.group("compute_sum") as compute_sum:
        add.left = lhs.out
        add.right = rhs.out
        sum.write_en = 1
        sum.in_ = add.out
        compute_sum.done = sum.done

    main.control += [
        update_operands,
        compute_sum,
    ]

    return prog.program


if __name__ == '__main__':
    build().emit()
