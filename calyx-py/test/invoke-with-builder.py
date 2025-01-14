import calyx.builder as cb

def insert_foo_component(prog):
    foo = prog.component("foo")

    a = foo.input("a", 32)
    foo.output("out", 32)

    temp = foo.reg(32, "temp")

    with foo.group("let", static_delay=1) as let:
        temp.in_ = a
        temp.write_en = cb.HI
        let.done = temp.done

    with foo.continuous:
        foo.this().out = temp.out

    foo.control += let

    return foo

def insert_main_component(prog):
    main = prog.component("main")

    # cells
    b = main.reg(32, "b")
    c = main.reg(32, "c")
    cst = main.const("cst", 32, 1)
    foo = main.cell("foo0", insert_foo_component(prog))

    # wires
    with main.group("write_constant", static_delay=1) as write_constant:
        b.in_ = cst.out
        b.write_en = cb.HI
        write_constant.done = b.done

    with main.group("save_foo") as save_foo:
        c.in_ = foo.out
        c.write_en = cb.HI
        save_foo.done = c.done

    main.control += write_constant
    main.control += cb.invoke(foo, in_a=b.out)
    main.control += save_foo
    

if __name__ == "__main__":
    prog = cb.Builder()
    insert_main_component(prog)
    prog.program.emit()
