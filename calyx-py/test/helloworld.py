import calyx.builder as cb


def insert_adder_component(prog):
    comp = prog.component("adder")

    val1 = comp.input("val1", 32)
    val2 = comp.input("val2", 32)
    comp.output("out", 32)

    sum = comp.reg(32)
    add = comp.add(32)

    with comp.group("compute_sum") as compute_sum:
        add.left = val1
        add.right = val2
        sum.write_en = cb.HI
        sum.in_ = add.out
        compute_sum.done = sum.done

    with comp.continuous:
        comp.this().out = sum.out

    comp.control += compute_sum


if __name__ == "__main__":
    prog = cb.Builder()
    insert_adder_component(prog)
    prog.program.emit()
