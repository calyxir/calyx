import calyx.builder as cb
from calyx import py_ast

# Global constant for the current bitwidth.
BITWIDTH = 32
INTWIDTH = 16
FRACWIDTH = 16
# Name of the pe component
PE_NAME = "mac_pe"


def pe(prog: cb.Builder):
    """
    Builds a pipelined "multiply and accumulate" PE that multiplies its `top`
    and `left` input values, and accumulate the value.
    The output is displayed through the `out` port.
    This PE can accept new inputs every cycle: therefore it has a `mul_ready`
    parameter to signal whether the output of the multiplier should be accumulated
    yet.
    """
    comp = prog.component(name=PE_NAME, latency=1)
    comp.input("top", BITWIDTH)
    comp.input("left", BITWIDTH)
    comp.input("mul_ready", 1)
    comp.output("out", BITWIDTH)
    acc = comp.reg("acc", BITWIDTH)
    add = comp.fp_sop("adder", "add", BITWIDTH, INTWIDTH, FRACWIDTH)
    mul = comp.pipelined_fp_smult("mul", BITWIDTH, INTWIDTH, FRACWIDTH)

    this = comp.this()
    with comp.static_group("do_add", 1):
        add.left = acc.out
        add.right = mul.out
        acc.in_ = add.out
        acc.write_en = this.mul_ready

    with comp.static_group("do_mul", 1):
        mul.left = this.top
        mul.right = this.left

    par = py_ast.StaticParComp([py_ast.Enable("do_add"), py_ast.Enable("do_mul")])

    with comp.continuous:
        this.out = acc.out

    comp.control += par
