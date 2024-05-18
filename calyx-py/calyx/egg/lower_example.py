from __future__ import annotations
import calyx.py_ast as ast
from calyx.builder import Builder, const, par, seq
import py_to_egg2

# ANCHOR: init


def add_main_component(prog, name):
    main = prog.component(name)
    main.input("in", 32)
    main.output("out", 32)
    # ANCHOR_END: init

    # ANCHOR: cells
    lhs = main.reg("r0", 32)
    rhs = main.reg("r1", 32)
    sum = main.reg("r2", 32)
    add = main.add(32, "a0")
    # ANCHOR_END: cells

    # ANCHOR: bare
    # Bare name of the cell
    add_out = ast.CompPort(ast.CompVar("add"), "out")
    # ANCHOR_END: bare

    # ANCHOR: group_def
    with main.group("A", static_delay=1010) as A:
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
        A.done = (lhs.done & rhs.done) @ 1
        # ANCHOR_END: done

    with main.group("B", static_delay=1) as B:
        add.left = lhs.out
        add.right = rhs.out
        sum.write_en = 1
        # Directly use the ast.CompPort object `add_out`.
        # This is useful HACK when we haven't defined the cell yet but still want
        # to use its ports.
        # ANCHOR: bare_use
        sum.in_ = add_out
        # ANCHOR_END: bare_use
        B.done = sum.done

    # ANCHOR: this
    # Use `this()` method to access the ports on the current component
    this = main.this()
    # ANCHOR_END: this
    # ANCHOR: continuous
    # ANCHOR_END: continuous

    # ANCHOR: control
    main.control = par(A, B, A, B, A, B, A, B, A, A, A, B, B, B, A, A, A)
    return main


# from egglog import *
# egraph = EGraph()
# lst = egraph.let("x", PyObject([ast.CompInst("a", [1,2,3])]))

# print(lst)
# print(egraph.eval(lst))

def run_example():
    b = Builder()
    add_main_component(b, "m1")
    c1 = b.get_component("m1").component

    cegg = py_to_egg2.CalyxEgg()
    cc1 = cegg.Convert(c1)

    try:
        cegg.saturate()
        ecc1_multiple = cegg.extract_multiple(cc1, 32)
        print()
        for x in ecc1_multiple:
            print(f":: {x}\n")

        print("best:")
        print(cegg.extract(ecc1))
        cegg.display()
    except Exception as e:
        print(f"uh-oh, failed: {e}")


run_example()
