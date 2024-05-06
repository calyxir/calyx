from typing import List
from math import log
from calyx.py_ast import (
    Stdlib,
    Component,
    Import,
)
from calyx.utils import float_to_fixed_point
from calyx import numeric_types
from calyx.gen_msb import gen_msb_calc

from calyx.builder import Builder, ComponentBuilder, CellBuilder, HI, par, invoke


def gen_constant_cell(
    comp: ComponentBuilder,
    name: str,
    value: str,
    width: int,
    int_width: int,
    is_signed: bool,
) -> CellBuilder:
    """
    Generates a constant cell named `name`, and value, width, int_width, and is_signed
    set according to their respective arguments.
    """
    return comp.const(
        name,
        width,
        numeric_types.FixedPoint(value, width, int_width, is_signed).unsigned_integer(),
    )


def multiply_cells(
    comp: ComponentBuilder, group_name: str, mult_cell: str, lhs: str, rhs: str
):
    """
    Returns a group named `group_name" that multiplies `lhs` and `rhs` using `mult_cell`
    """
    mult_cell_actual = comp.get_cell(mult_cell)
    lhs_actual = comp.get_cell(lhs)
    rhs_actual = comp.get_cell(rhs)

    with comp.group(group_name) as group:
        mult_cell_actual.go = HI
        mult_cell_actual.left = rhs_actual.out
        mult_cell_actual.right = lhs_actual.out
        group.done = mult_cell_actual.done


def generate_pade_cells(
    comp: ComponentBuilder, width: int, int_width: int, is_signed: bool
):
    """
    Generates cells for pade approximant component
    """
    frac_width = width - int_width
    gen_constant_cell(
        comp,
        "n1",
        str(float_to_fixed_point(3.40547, frac_width)),
        width,
        int_width,
        is_signed,
    )
    gen_constant_cell(
        comp,
        "n2",
        str(float_to_fixed_point(2.43279, frac_width)),
        width,
        int_width,
        is_signed,
    )
    gen_constant_cell(
        comp,
        "n3",
        str(float_to_fixed_point(5.8376, frac_width)),
        width,
        int_width,
        is_signed,
    )
    gen_constant_cell(
        comp,
        "d2",
        str(float_to_fixed_point(6.0, frac_width)),
        width,
        int_width,
        is_signed,
    )
    gen_constant_cell(
        comp,
        "d3",
        str(float_to_fixed_point(2.25, frac_width)),
        width,
        int_width,
        is_signed,
    )
    comp.cell(
        "mult_pipe",
        Stdlib.fixed_point_op(
            "mult_pipe", width, int_width, width - int_width, is_signed
        ),
    )
    comp.cell(
        "n_mult_pipe1",
        Stdlib.fixed_point_op(
            "mult_pipe", width, int_width, width - int_width, is_signed
        ),
    )
    comp.cell(
        "n_mult_pipe2",
        Stdlib.fixed_point_op(
            "mult_pipe", width, int_width, width - int_width, is_signed
        ),
    )
    comp.cell(
        "d_mult_pipe1",
        Stdlib.fixed_point_op(
            "mult_pipe", width, int_width, width - int_width, is_signed
        ),
    )
    comp.cell(
        "d_mult_pipe2",
        Stdlib.fixed_point_op(
            "mult_pipe", width, int_width, width - int_width, is_signed
        ),
    )
    comp.cell(
        "div_pipe",
        Stdlib.fixed_point_op(
            "div_pipe", width, int_width, width - int_width, is_signed
        ),
    )
    comp.cell(
        "add1",
        Stdlib.fixed_point_op("add", width, int_width, width - int_width, is_signed),
    )
    comp.cell(
        "add2",
        Stdlib.fixed_point_op("add", width, int_width, width - int_width, is_signed),
    )
    comp.cell(
        "add3",
        Stdlib.fixed_point_op("add", width, int_width, width - int_width, is_signed),
    )
    comp.cell(
        "sub1",
        Stdlib.fixed_point_op("sub", width, int_width, width - int_width, is_signed),
    )

    comp.reg(width, "num_reg")
    comp.reg(width, "den_reg")
    comp.reg(width, "res_reg")
    comp.reg(width, "x_reg")
    comp.reg(width, "x_sq_reg")


def generate_pade_groups(comp: ComponentBuilder):
    """
    Generates groups for pade approximant componenet
    """

    multiply_cells(comp, "get_x_sq", "mult_pipe", "x_reg", "x_reg"),
    multiply_cells(comp, "num_term1", "n_mult_pipe1", "mult_pipe", "n1"),
    multiply_cells(comp, "num_term2", "n_mult_pipe2", "x_reg", "n2"),
    multiply_cells(comp, "den_term2", "d_mult_pipe2", "x_reg", "d2"),

    x_reg = comp.get_cell("x_reg")
    add1 = comp.get_cell("add1")
    add2 = comp.get_cell("add2")
    add3 = comp.get_cell("add3")
    n_mult_pipe1 = comp.get_cell("n_mult_pipe1")
    n_mult_pipe2 = comp.get_cell("n_mult_pipe2")
    d_mult_pipe2 = comp.get_cell("d_mult_pipe2")
    sub1 = comp.get_cell("sub1")
    n3 = comp.get_cell("n3")
    d3 = comp.get_cell("d3")
    num_reg = comp.get_cell("num_reg")
    den_reg = comp.get_cell("den_reg")
    mult_pipe = comp.get_cell("mult_pipe")
    div_pipe = comp.get_cell("div_pipe")
    res_reg = comp.get_cell("res_reg")

    with comp.group("write_x_to_reg") as write_x_to_reg:
        x_reg.write_en = HI
        x_reg.in_ = comp.this().x
        write_x_to_reg.done = x_reg.done

    with comp.group("get_numerator") as get_numerator:
        add1.left = n_mult_pipe1.out
        add1.right = n_mult_pipe2.out
        sub1.left = add1.out
        sub1.right = n3.out
        num_reg.in_ = sub1.out
        num_reg.write_en = HI
        get_numerator.done = num_reg.done

    with comp.group("get_denominator") as get_denominator:
        add2.left = mult_pipe.out
        add2.right = d_mult_pipe2.out
        add3.left = add2.out
        add3.right = d3.out
        den_reg.in_ = add3.out
        den_reg.write_en = HI
        get_denominator.done = den_reg.done

    with comp.group("get_res") as get_res:
        res_reg.write_en = HI
        res_reg.in_ = div_pipe.out_quotient
        get_res.done = res_reg.done


def gen_pade_approx(width: int, int_width: int, is_signed: bool) -> List[Component]:
    """
    Component to approximate ln(x).
    Uses the 2nd order Pade Approximant of ln(x) at x = 1.5. Therefore, we only
    us this component when 1 <= x < 2.
    Formula calculated using Wolfram Alpha:
    https://www.wolframalpha.com/input?i=+PadeApproximant%5Bln%28x%29%2C%7Bx%2C1.5%2C%7B2%2C2%7D%7D%5D+
    Read About Pade Approximant here:
    https://en.wikipedia.org/wiki/Pad%C3%A9_approximant
    """
    builder = Builder()
    comp = builder.component("ln_pade_approx")
    comp.input("x", width)
    comp.output("out", width)

    generate_pade_cells(comp, width, int_width, is_signed)
    generate_pade_groups(comp)
    with comp.continuous:
        comp.this().out = comp.get_cell("res_reg").out

    comp.control += [
        comp.get_group("write_x_to_reg"),
        comp.get_group("get_x_sq"),
        par(
            comp.get_group("num_term1"),
            comp.get_group("num_term2"),
            comp.get_group("den_term2"),
        ),
        par(comp.get_group("get_numerator"), comp.get_group("get_denominator")),
        invoke(
            comp.get_cell("div_pipe"),
            in_left=comp.get_cell("num_reg").out,
            in_right=comp.get_cell("den_reg").out,
        ),
        comp.get_group("get_res"),
    ]

    return [comp.component]


def generate_ln(width: int, int_width: int, is_signed: bool) -> List[Component]:
    """
    Generates a component that approximates ln(x) for x >= 1.
    Notice that x = 2^n * y for some natural number n, and some p between 1 and 2.
    Therefore, ln(x) = ln(2^n * p) = ln(2^n) + ln(p) = n*ln(2) + ln(p).
    Therefore, we can calculate 2 * ln(2) easily (since we can just store ln(2)
    as a constant),and then add ln(p) using the pade approximant.
    We use the `msb_calc` component (located in gen_msb.py) to calculate the n and p values.
    """
    builder = Builder()
    comp = builder.component("ln")
    comp.input("x", width)
    comp.output("out", width)

    # this is unused for some reason
    and1 = comp.cell("and1", Stdlib.op("and", width, signed=False))

    n = comp.reg(width, "n")
    div_pipe = comp.cell(
        "div_pipe",
        Stdlib.fixed_point_op(
            "div_pipe", width, int_width, width - int_width, is_signed
        ),
    )
    add1 = comp.cell(
        "add1",
        Stdlib.fixed_point_op("add", width, int_width, width - int_width, is_signed),
    )
    mult_pipe = comp.cell(
        "mult_pipe",
        Stdlib.fixed_point_op(
            "mult_pipe", width, int_width, width - int_width, is_signed
        ),
    )
    ln_2 = gen_constant_cell(
        comp,
        "ln_2",
        str(float_to_fixed_point(log(2), width - int_width)),
        width,
        int_width,
        is_signed,
    )
    pade_approx = comp.comp_instance(
        "pade_approx", "ln_pade_approx", check_undeclared=False
    )
    res_reg = comp.reg(width, "res_reg")
    msb = comp.comp_instance("msb", "msb_calc", check_undeclared=False)
    # these 3 appear unused, not sure why
    slice0 = comp.cell("slice0", Stdlib.slice(width, int_width))
    rsh = comp.cell("rsh", Stdlib.op("rsh", width, is_signed))
    shift_amount = comp.const("shift_amount", width, int_width)

    with comp.group("get_n") as get_n:
        n.write_en = HI
        n.in_ = msb.count
        get_n.done = n.done

    with comp.group("get_p") as get_p:
        div_pipe.go = HI
        div_pipe.left = comp.this().x
        div_pipe.right = msb.value
        get_p.done = div_pipe.done

    with comp.group("get_term1") as get_term1:
        mult_pipe.go = HI
        mult_pipe.left = ln_2.out
        mult_pipe.right = n.out
        get_term1.done = mult_pipe.done

    with comp.group("get_res") as get_res:
        add1.left = mult_pipe.out
        add1.right = pade_approx.out
        res_reg.in_ = add1.out
        res_reg.write_en = HI
        get_res.done = res_reg.done

    with comp.continuous:
        comp.this().out = res_reg.out

    comp.control += [
        invoke(
            msb,
            in_in=comp.this().x,
        ),
        get_n,
        get_p,
        get_term1,
        invoke(
            pade_approx,
            in_x=div_pipe.out_quotient,
        ),
        get_res,
    ]

    return (
        gen_pade_approx(width, int_width, is_signed)
        + gen_msb_calc(width, int_width)
        + [comp.component]
    )


if __name__ == "__main__":
    import argparse
    import json

    parser = argparse.ArgumentParser(
        description="`exp` using a Taylor Series approximation"
    )
    parser.add_argument("file", nargs="?", type=str)
    parser.add_argument("-w", "--width", type=int)
    parser.add_argument("-i", "--int_width", type=int)

    args = parser.parse_args()

    width, int_width = None, None
    required_fields = [args.width, args.int_width]
    if all(map(lambda x: x is not None, required_fields)):
        width = args.width
        int_width = args.int_width
    elif args.file is not None:
        with open(args.file, "r") as f:
            spec = json.load(f)
            is_signed = spec["is_signed"]
            width = spec["width"]
            int_width = spec["int_width"]
    else:
        parser.error(
            "Need to pass either `-f FILE` or all of `-d DEGREE -w WIDTH -i INT_WIDTH`"
        )

    # NOTE (griffin): I'm gonna leave these but I am pretty sure this is a copy
    # paste error
    # build 2 separate programs: 1 if base_is_e is true, the other if false
    # any_base_program is (obviously) the one for any base

    builder = Builder()
    builder.program.imports += [
        Import("primitives/core.futil"),
        Import("primitives/binary_operators.futil"),
    ]
    builder.program.components.append(generate_ln(width, int_width, is_signed))

    # main component for testing purposes
    main = builder.component("main")
    x = main.reg(width, "x")
    in_ = main.comb_mem_d1("in", width, 1, 1, is_external=True)
    out = main.comb_mem_d1("out", width, 1, 1, is_external=True)
    ln = main.comp_instance("l", "ln")

    with main.group("read_in_mem") as read_in_mem:
        in_.read_addr = 0
        x.in_ = in_.read_data
        x.write_en = HI
        read_in_mem.done = x.done

    with main.group("write_to_memory") as write_to_memory:
        out.addr0 = 0
        out.write_en = HI
        out.write_data = ln.out
        write_to_memory.done = out.done

    main.control += [
        read_in_mem,
        invoke(
            ln,
            in_x=x.out,
        ),
        write_to_memory,
    ]

    builder.program.emit()
