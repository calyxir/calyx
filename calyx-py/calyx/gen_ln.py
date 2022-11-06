from typing import List
from math import log2, log
from calyx.py_ast import (
    Connect, CompVar, Cell, Group, ConstantPort, CompPort, Stdlib,
    Component, ThisPort, And, HolePort, Atom, Not, PortDef, SeqComp,
    Enable, While, ParComp, Structure, CompInst, Invoke, Program, Control,
    If, Import, CombGroup
)
from calyx.utils import float_to_fixed_point
from fud.stages.verilator import numeric_types
from calyx.gen_msb import gen_msb_calc


def gen_constant_cell(name, value, width, int_width, is_signed) -> Cell:
    '''
    Generates a constant cell named `name`, and value, widht, int_width, and is_signed 
    set according to their respective arguments. 
    '''
    stdlib = Stdlib()
    return Cell(
        CompVar(name),
        stdlib.constant(
            width,
            numeric_types.FixedPoint(
                value, width, int_width, is_signed
            ).unsigned_integer(),
        ),
    )


def multiply_cells(group_name, mult_cell, lhs, rhs):
    '''
    Returns a group named `group_name" that multiplies `lhs` and `rhs` using `mult_cell`
    '''
    return Group(
        id=CompVar(group_name),
        connections=[
            Connect(
                CompPort(CompVar(mult_cell), "go"),
                ConstantPort(1, 1),
            ),
            Connect(
                CompPort(CompVar(mult_cell), "left"),
                CompPort(CompVar(rhs), "out"),
            ),
            Connect(
                CompPort(CompVar(mult_cell), "right"),
                CompPort(CompVar(lhs), "out"),
            ),
            Connect(HolePort(CompVar(group_name), "done"),
                    CompPort(CompVar(mult_cell), "done"))
        ]
    )


def generate_pade_cells(width: int, int_width: int, is_signed: bool) -> List[Cell]:
    '''
    Generates cells for pade approximant componenet
    '''
    frac_width = width - int_width
    n1 = gen_constant_cell("n1", str(float_to_fixed_point(3.40547, frac_width)),
                           width, int_width, is_signed)
    n2 = gen_constant_cell("n2", str(float_to_fixed_point(2.43279, frac_width)),
                           width, int_width, is_signed)
    n3 = gen_constant_cell("n3", str(float_to_fixed_point(5.8376, frac_width)),
                           width, int_width, is_signed)
    d2 = gen_constant_cell("d2", str(float_to_fixed_point(6.0, frac_width)),
                           width, int_width, is_signed)
    d3 = gen_constant_cell("d3", str(float_to_fixed_point(2.25, frac_width)),
                           width, int_width, is_signed)
    mult_pipe = Cell(CompVar("mult_pipe"), Stdlib().fixed_point_op(
        "mult_pipe", width, int_width, width-int_width, is_signed))
    n_mult_pipe1 = Cell(CompVar("n_mult_pipe1"), Stdlib().fixed_point_op(
        "mult_pipe", width, int_width, width-int_width, is_signed))
    n_mult_pipe2 = Cell(CompVar("n_mult_pipe2"), Stdlib().fixed_point_op(
        "mult_pipe", width, int_width, width-int_width, is_signed))
    d_mult_pipe1 = Cell(CompVar("d_mult_pipe1"), Stdlib().fixed_point_op(
        "mult_pipe", width, int_width, width-int_width, is_signed))
    d_mult_pipe2 = Cell(CompVar("d_mult_pipe2"), Stdlib().fixed_point_op(
        "mult_pipe", width, int_width, width-int_width, is_signed))
    div_pipe = Cell(CompVar("div_pipe"), Stdlib().fixed_point_op(
        "div_pipe", width, int_width, width-int_width, is_signed))
    add1 = Cell(CompVar("add1"), Stdlib().fixed_point_op(
        "add", width, int_width, width-int_width, is_signed))
    add2 = Cell(CompVar("add2"), Stdlib().fixed_point_op(
        "add", width, int_width, width-int_width, is_signed))
    add3 = Cell(CompVar("add3"), Stdlib().fixed_point_op(
        "add", width, int_width, width-int_width, is_signed))
    sub1 = Cell(CompVar("sub1"), Stdlib().fixed_point_op(
        "sub", width, int_width, width-int_width, is_signed))
    numerator_reg = Cell(CompVar("num_reg"), Stdlib().register(width))
    denominator_reg = Cell(CompVar("den_reg"), Stdlib().register(width))
    res_reg = Cell(CompVar("res_reg"), Stdlib().register(width))
    x_reg = Cell(CompVar("x_reg"), Stdlib().register(width))
    x_sq_reg = Cell(CompVar("x_sq_reg"), Stdlib().register(width))
    return [n1, n2, n3, d2, d3, mult_pipe, n_mult_pipe1, n_mult_pipe2, d_mult_pipe1, d_mult_pipe2, div_pipe, add1, add2, add3, sub1, numerator_reg, denominator_reg, res_reg, x_reg, x_sq_reg]


def generate_pade_groups() -> List[Group]:
    '''
    Generates groups for pade approximant componenet
    '''
    mult_groups = [multiply_cells("get_x_sq", "mult_pipe", "x_reg", "x_reg"),
                   multiply_cells("num_term1", "n_mult_pipe1", "mult_pipe", "n1"),
                   multiply_cells("num_term2", "n_mult_pipe2", "x_reg", "n2"),
                   multiply_cells("den_term2", "d_mult_pipe2", "x_reg", "d2"),
                   ]
    write_x_to_reg = Group(
        id=CompVar("write_x_to_reg"),
        connections=[
            Connect(
                CompPort(CompVar("x_reg"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(
                CompPort(CompVar("x_reg"), "in"),
                ThisPort(CompVar("x"))
            ),
            Connect(HolePort(CompVar("write_x_to_reg"), "done"),
                    CompPort(CompVar("x_reg"), "done"))
        ]
    )
    get_numerator = Group(
        id=CompVar("get_numerator"),
        connections=[
            Connect(
                CompPort(CompVar("add1"), "left"),
                CompPort(CompVar("n_mult_pipe1"), "out"),
            ),
            Connect(
                CompPort(CompVar("add1"), "right"),
                CompPort(CompVar("n_mult_pipe2"), "out"),
            ),
            Connect(
                CompPort(CompVar("sub1"), "left"),
                CompPort(CompVar("add1"), "out"),
            ),
            Connect(
                CompPort(CompVar("sub1"), "right"),
                CompPort(CompVar("n3"), "out"),
            ),
            Connect(
                CompPort(CompVar("num_reg"), "in"),
                CompPort(CompVar("sub1"), "out"),
            ),
            Connect(
                CompPort(CompVar("num_reg"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(
                HolePort(CompVar("get_numerator"), "done"),
                CompPort(CompVar("num_reg"), "done"),
            )
        ]
    )
    get_denominator = Group(
        id=CompVar("get_denominator"),
        connections=[
            Connect(
                CompPort(CompVar("add2"), "left"),
                CompPort(CompVar("mult_pipe"), "out"),
            ),
            Connect(
                CompPort(CompVar("add2"), "right"),
                CompPort(CompVar("d_mult_pipe2"), "out"),
            ),
            Connect(
                CompPort(CompVar("add3"), "left"),
                CompPort(CompVar("add2"), "out"),
            ),
            Connect(
                CompPort(CompVar("add3"), "right"),
                CompPort(CompVar("d3"), "out"),
            ),
            Connect(
                CompPort(CompVar("den_reg"), "in"),
                CompPort(CompVar("add3"), "out"),
            ),
            Connect(
                CompPort(CompVar("den_reg"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(
                HolePort(CompVar("get_denominator"), "done"),
                CompPort(CompVar("den_reg"), "done"),
            )
        ]
    )
    get_res = Group(
        id=CompVar("get_res"),
        connections=[
            Connect(
                CompPort(CompVar("res_reg"), "write_en"),
                ConstantPort(1, 1)
            ),
            Connect(
                CompPort(CompVar("res_reg"), "in"),
                CompPort(CompVar("div_pipe"), "out_quotient"),
            ),
            Connect(
                HolePort(CompVar("get_res"), "done"),
                CompPort(CompVar("res_reg"), "done"),
            ),
        ]
    )
    return mult_groups + [write_x_to_reg, get_numerator, get_denominator, get_res]


def generate_pade_control() -> Control:
    '''
    Generates control for pade approximant componenet
    '''
    return SeqComp([
        Enable("write_x_to_reg"),
        Enable("get_x_sq"),
        ParComp([Enable("num_term1"), Enable("num_term2"), Enable("den_term2")]),
        ParComp([Enable("get_numerator"), Enable("get_denominator")]),
        Invoke(CompVar("div_pipe"), [("left",  CompPort(
            CompVar("num_reg"), "out")), ("right", CompPort(CompVar("den_reg"), "out"))], []),
        Enable("get_res")
    ])


def gen_pade_approx(width: int, int_width: int, is_signed: bool) -> List[Component]:
    '''
    Component to approximate ln(x).
    Uses the 2nd order Pade Approximant of ln(x) at x = 1.5. Therefore, we only 
    us this component when 1 <= x < 2. 
    Formula calculated using Wolfram Alpha:
    https://www.wolframalpha.com/input?i=+PadeApproximant%5Bln%28x%29%2C%7Bx%2C1.5%2C%7B2%2C2%7D%7D%5D+
    Read About Pade Approximant here:
    https://en.wikipedia.org/wiki/Pad%C3%A9_approximant
    '''
    return [Component(
            "ln_pade_approx",
            inputs=[PortDef(CompVar("x"), width)],
            outputs=[PortDef(CompVar("out"), width)],
            structs=generate_pade_cells(
                width, int_width, is_signed) + generate_pade_groups()
            + [Connect(ThisPort(CompVar("out")),
                       CompPort(CompVar("res_reg"), "out"))],
            controls=generate_pade_control(),
            )]


def gen_ln_cells(width: int, int_width: int, is_signed: bool) -> List[Cell]:
    '''
    Generates cells for the ln component.
    '''
    stdlib = Stdlib()
    and1 = Cell(CompVar("and1"), stdlib.op("and", width, signed=False))
    n = Cell(CompVar("n"), Stdlib().register(width))
    div_pipe = Cell(CompVar("div_pipe"), Stdlib().fixed_point_op(
        "div_pipe", width, int_width, width-int_width, is_signed))
    add1 = Cell(CompVar("add1"), Stdlib().fixed_point_op(
        "add", width, int_width, width-int_width, is_signed))
    mult_pipe = Cell(CompVar("mult_pipe"), Stdlib().fixed_point_op(
        "mult_pipe", width, int_width, width-int_width, is_signed))
    ln_2 = gen_constant_cell(
        "ln_2", str(float_to_fixed_point(log(2), width-int_width)),
        width, int_width, is_signed)
    pade_approx = Cell(CompVar("pade_approx"), CompInst("ln_pade_approx", []))
    res_reg = Cell(CompVar("res_reg"), Stdlib().register(width))
    msb_gen = Cell(CompVar("msb"), CompInst("msb_calc", []))
    slice0 = Cell(CompVar("slice0"), Stdlib().slice(width, int_width))
    rsh = Cell(CompVar("rsh"), Stdlib().op("rsh", width, is_signed))
    shift_amount = Cell(CompVar("shift_amount"), stdlib.constant(width, int_width))
    return [and1, n, div_pipe, mult_pipe, ln_2, pade_approx, res_reg, add1, msb_gen, slice0, rsh, shift_amount]


def gen_ln_groups() -> List[Group]:
    '''
    Generates groups for the ln component
    '''
    get_n = Group(
        id=CompVar("get_n"),
        connections=[
            Connect(
                CompPort(CompVar("n"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(
                CompPort(CompVar("n"), "in"),
                CompPort(CompVar("msb"), "count"),
            ),
            Connect(
                HolePort(CompVar("get_n"), "done"),
                CompPort(CompVar("n"), "done"),
            ),
        ],
    )

    get_p = Group(
        id=CompVar("get_p"),
        connections=[
            Connect(
                CompPort(CompVar("div_pipe"), "go"),
                ConstantPort(1, 1),
            ),
            Connect(
                CompPort(CompVar("div_pipe"), "left"),
                ThisPort(CompVar("x")),
            ),
            Connect(
                CompPort(CompVar("div_pipe"), "right"),
                CompPort(CompVar("msb"), "value"),
            ),
            Connect(HolePort(CompVar("get_p"), "done"),
                    CompPort(CompVar("div_pipe"), "done"))
        ]
    )

    get_term_1 = Group(
        id=CompVar("get_term1"),
        connections=[
            Connect(
                CompPort(CompVar("mult_pipe"), "go"),
                ConstantPort(1, 1),
            ),
            Connect(
                CompPort(CompVar("mult_pipe"), "left"),
                CompPort(CompVar("ln_2"), "out"),
            ),
            Connect(
                CompPort(CompVar("mult_pipe"), "right"),
                CompPort(CompVar("n"), "out"),
            ),
            Connect(HolePort(CompVar("get_term1"), "done"),
                    CompPort(CompVar("mult_pipe"), "done"))
        ]
    )

    get_res = Group(
        id=CompVar("get_res"),
        connections=[
            Connect(
                CompPort(CompVar("add1"), "left"),
                CompPort(CompVar("mult_pipe"), "out"),
            ),
            Connect(
                CompPort(CompVar("add1"), "right"),
                CompPort(CompVar("pade_approx"), "out"),
            ),
            Connect(
                CompPort(CompVar("res_reg"), "in"),
                CompPort(CompVar("add1"), "out"),
            ),
            Connect(
                CompPort(CompVar("res_reg"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(HolePort(CompVar("get_res"), "done"),
                    CompPort(CompVar("res_reg"), "done"))
        ]
    )

    return [get_n, get_p, get_term_1, get_res]


def gen_ln_control() -> Control:
    '''
    Generates control for the ln component
    '''
    return SeqComp(
        [
            Invoke(id=CompVar("msb"), in_connects=[
                   ("in",  ThisPort(CompVar("x")))], out_connects=[]),
            Enable("get_n"),
            Enable("get_p"),
            Enable("get_term1"),
            Invoke(
                id=CompVar("pade_approx"),
                in_connects=[("x", CompPort(CompVar("div_pipe"), "out_quotient"))],
                out_connects=[],
            ),
            Enable("get_res")
        ]
    )


def generate_ln(width: int, int_width: int, is_signed: bool) -> List[Component]:
    """
    Generates a component that approximates ln(x) for x >= 1.
    Notice that x = 2^n * y for some natural number n, and some p between 1 and 2.
    Therefore, ln(x) = ln(2^n * p) = ln(2^n) + ln(p) = n*ln(2) + ln(p).
    Therefore, we can calculate 2 * ln(2) easily (since we can just store ln(2) 
    as a constant),and then add ln(p) using the pade approximant.
    We use the `msb_calc` component (located in gen_msb.py) to calculate the n and p values.
    """
    return (gen_pade_approx(width, int_width, is_signed) +
            gen_msb_calc(width, int_width) +
            [
        Component(
            "ln",
            inputs=[PortDef(CompVar("x"), width)],
            outputs=[PortDef(CompVar("out"), width)],
            structs=gen_ln_cells(width, int_width, is_signed)
            + gen_ln_groups() + [Connect(ThisPort(CompVar("out")),
                                         CompPort(CompVar("res_reg"), "out"))],
            controls=gen_ln_control(),
        )
    ])


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

    # build 2 separate programs: 1 if base_is_e is true, the other if false
    # any_base_program is (obviously) the one for any base
    program = Program(
        imports=[Import("primitives/core.futil"),
                 Import("primitives/binary_operators.futil")],
        components=generate_ln(width, int_width, is_signed)
    )
    # main component for testing purposes
    main = Component(
        "main",
        inputs=[],
        outputs=[],
        structs=[
            Cell(CompVar("x"), Stdlib().register(width)),
            Cell(CompVar("in"), Stdlib().mem_d1(
                width, 1, 1), is_external=True),
            Cell(
                CompVar("out"),
                Stdlib().mem_d1(width, 1, 1),
                is_external=True,
            ),
            Cell(CompVar("l"), CompInst("ln", [])),
            Group(
                id=CompVar("read_in_mem"),
                connections=[
                    Connect(
                        CompPort(CompVar("in"), "addr0"),
                        ConstantPort(1, 0),
                    ),
                    Connect(
                        CompPort(CompVar("x"), "in"),
                        CompPort(CompVar("in"), "read_data"),
                    ),
                    Connect(
                        CompPort(CompVar("x"), "write_en"),
                        ConstantPort(1, 1),
                    ),
                    Connect(
                        HolePort(CompVar("read_in_mem"), "done"),
                        CompPort(CompVar("x"), "done"),
                    ),
                ],
            ),
            Group(
                id=CompVar("write_to_memory"),
                connections=[
                    Connect(
                        CompPort(CompVar("out"), "addr0"),
                        ConstantPort(1, 0),
                    ),
                    Connect(
                        CompPort(CompVar("out"), "write_en"),
                        ConstantPort(1, 1),
                    ),
                    Connect(
                        CompPort(CompVar("out"), "write_data"),
                        CompPort(CompVar("l"), "out"),
                    ),
                    Connect(
                        HolePort(CompVar("write_to_memory"), "done"),
                        CompPort(CompVar("out"), "done"),
                    ),
                ],
            ),
        ],
        controls=SeqComp(
            [
                Enable("read_in_mem"),
                Invoke(
                    id=CompVar("l"),
                    in_connects=[("x", CompPort(CompVar("x"), "out"))],
                    out_connects=[],
                ),
                Enable("write_to_memory"),
            ]
        ),
    )
    program.components.append(main)

    program.emit()
