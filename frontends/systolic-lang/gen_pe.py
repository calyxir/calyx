import calyx.builder as cb

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
    top = comp.input("top", BITWIDTH)
    left = comp.input("left", BITWIDTH)
    mul_ready = comp.input("mul_ready", 1)
    comp.output("out", BITWIDTH)
    acc = comp.reg(BITWIDTH, "acc")
    add = comp.fp_sop("adder", "add", BITWIDTH, INTWIDTH, FRACWIDTH)
    mul = comp.pipelined_fp_smult("mul", BITWIDTH, INTWIDTH, FRACWIDTH)

    with comp.static_group("do_add", 1) as do_add:
        add.left = acc.out
        add.right = mul.out
        acc.in_ = add.out
        acc.write_en = mul_ready

    with comp.static_group("do_mul", 1) as do_mul:
        mul.left = top
        mul.right = left

    with comp.continuous:
        comp.this().out = acc.out

    comp.control += cb.static_par(do_add, do_mul)
