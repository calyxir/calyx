from calyx.py_ast import (
    Connect, CompVar, Cell, Group, ConstantPort, CompPort, Stdlib,
    Component, ThisPort, And, HolePort, Atom, Not, PortDef, SeqComp,
    Enable, While, ParComp, Structure, CompInst, Invoke, Program, Control,
    If, Import, CombGroup
)
from calyx.utils import float_to_fixed_point
from math import factorial, log2
from typing import List
from fud.stages.verilator import numeric_types


def generate_fp_pow_component(
        width: int, int_width: int, is_signed: bool) -> Component:
    """Generates a fixed point `pow` component, which
    computes the value x**y, where y must be an integer.
    """
    stdlib = Stdlib()
    frac_width = width - int_width

    pow = CompVar("pow")
    count = CompVar("count")
    mul = CompVar("mul")
    lt = CompVar("lt")
    incr = CompVar("incr")

    cells = [
        Cell(pow, stdlib.register(width)),
        Cell(count, stdlib.register(width)),
        Cell(
            mul,
            stdlib.fixed_point_op(
                "mult_pipe", width, int_width, frac_width, signed=is_signed
            ),
        ),
        Cell(lt, stdlib.op("lt", width, signed=is_signed)),
        Cell(incr, stdlib.op("add", width, signed=is_signed)),
    ]
    wires = [
        Group(
            id=CompVar("init"),
            connections=[
                Connect(
                    CompPort(pow, "in"),
                    ConstantPort(
                        width,
                        numeric_types.FixedPoint(
                            "1.0", width, int_width, is_signed=is_signed
                        ).unsigned_integer(),
                    ),
                ),
                Connect(CompPort(pow, "write_en"), ConstantPort(1, 1)),
                Connect(CompPort(count, "in"), ConstantPort(width, 0)),
                Connect(CompPort(count, "write_en"), ConstantPort(1, 1)),
                Connect(
                    HolePort(CompVar("init"), "done"),
                    ConstantPort(1, 1),
                    And(
                        Atom(CompPort(pow, "done")),
                        Atom(CompPort(count, "done")),
                    ),
                ),
            ],
        ),
        Group(
            id=CompVar("execute_mul"),
            connections=[
                Connect(CompPort(mul, "left"), ThisPort(CompVar("base"))),
                Connect(CompPort(mul, "right"), CompPort(pow, "out")),
                Connect(
                    CompPort(mul, "go"),
                    ConstantPort(1, 1),
                    Not(Atom(CompPort(mul, "done"))),
                ),
                Connect(CompPort(pow, "write_en"), CompPort(mul, "done")),
                Connect(CompPort(pow, "in"), CompPort(mul, "out")),
                Connect(
                    HolePort(CompVar("execute_mul"), "done"),
                    CompPort(pow, "done"),
                ),
            ],
        ),
        Group(
            id=CompVar("incr_count"),
            connections=[
                Connect(CompPort(incr, "left"), ConstantPort(width, 1)),
                Connect(CompPort(incr, "right"), CompPort(count, "out")),
                Connect(CompPort(count, "in"), CompPort(incr, "out")),
                Connect(CompPort(count, "write_en"), ConstantPort(1, 1)),
                Connect(
                    HolePort(CompVar("incr_count"), "done"),
                    CompPort(count, "done"),
                ),
            ],
        ),
        CombGroup(
            id=CompVar("cond"),
            connections=[
                Connect(CompPort(lt, "left"), CompPort(count, "out")),
                Connect(CompPort(lt, "right"),
                        ThisPort(CompVar("integer_exp"))),
            ],
        ),
        Connect(ThisPort(CompVar("out")), CompPort(CompVar("pow"), "out")),
    ]
    return Component(
        "fp_pow",
        inputs=[
            PortDef(CompVar("base"), width),
            PortDef(CompVar("integer_exp"), width),
        ],
        outputs=[PortDef(CompVar("out"), width)],
        structs=cells + wires,
        controls=SeqComp(
            [
                Enable("init"),
                While(
                    CompPort(lt, "out"),
                    CompVar("cond"),
                    ParComp([Enable("execute_mul"), Enable("incr_count")]),
                ),
            ]
        ),
    )


def generate_cells(
    degree: int, width: int, int_width: int, is_signed: bool
) -> List[Cell]:
    stdlib = Stdlib()
    frac_width = width - int_width
    init_cells = [
        Cell(CompVar("exponent_value"), stdlib.register(width)),
        Cell(CompVar("int_x"), stdlib.register(width)),
        Cell(CompVar("frac_x"), stdlib.register(width)),
        Cell(CompVar("m"), stdlib.register(width)),
        Cell(CompVar("and0"), stdlib.op("and", width, signed=False)),
        Cell(CompVar("and1"), stdlib.op("and", width, signed=False)),
        Cell(CompVar("rsh"), stdlib.op("rsh", width, signed=False)),
    ] + (
        [Cell(CompVar("lt"), stdlib.op("lt", width, signed=is_signed))]
        if is_signed
        else []
    )

    pow_registers = [
        Cell(
            CompVar(f"p{i}"),
            stdlib.register(width)
        ) for i in range(2, degree + 1)
    ]
    product_registers = [
        Cell(CompVar(f"product{i}"), stdlib.register(width))
        for i in range(2, degree + 1)
    ]
    sum_registers = [
        Cell(CompVar(f"sum{i}"), stdlib.register(width))
        for i in range(1, (degree // 2) + 1)
    ]
    adds = [
        Cell(
            CompVar(f"add{i}"),
            stdlib.fixed_point_op(
                "add", width, int_width, frac_width, signed=is_signed
            ),
        )
        for i in range(1, (degree // 2) + 1)
    ]
    pipes = [
        Cell(
            CompVar(f"mult_pipe{i}"),
            stdlib.fixed_point_op(
                "mult_pipe", width, int_width, frac_width, signed=is_signed
            ),
        )
        for i in range(1, degree + 1)
    ]
    # One extra `fp_pow` instance to compute e^{int_value}.
    pows = [
        Cell(
            CompVar(f"pow{i}"), CompInst("fp_pow", [])
        ) for i in range(1, degree + 1)
    ]
    reciprocal_factorials = []
    for i in range(2, degree + 1):
        fixed_point_value = float_to_fixed_point(
            1.0 / factorial(i), frac_width)
        value = numeric_types.FixedPoint(
            str(fixed_point_value), width, int_width, is_signed=is_signed
        ).unsigned_integer()
        reciprocal_factorials.append(
            Cell(CompVar(f"reciprocal_factorial{i}"), stdlib.constant(
                width, value))
        )
    # Constant values for the exponent to the fixed point `pow` function.
    constants = [
        Cell(
            CompVar(f"c{i}"), stdlib.constant(width, i)
        ) for i in range(2, degree + 1)
    ] + [
        Cell(
            CompVar("one"),
            stdlib.constant(
                width,
                numeric_types.FixedPoint(
                    "1.0", width, int_width, is_signed=is_signed
                ).unsigned_integer(),
            ),
        ),
        Cell(
            CompVar("e"),
            stdlib.constant(
                width,
                numeric_types.FixedPoint(
                    str(float_to_fixed_point(2.7182818284, frac_width)),
                    width,
                    int_width,
                    is_signed=is_signed,
                ).unsigned_integer(),
            ),
        ),
    ]
    if is_signed:
        constants.append(
            Cell(
                CompVar("negative_one"),
                stdlib.constant(
                    width,
                    numeric_types.FixedPoint(
                        "-1.0", width, int_width, is_signed=is_signed
                    ).unsigned_integer(),
                ),
            ),
        )
        pipes.append(
            Cell(
                CompVar("div_pipe"),
                stdlib.fixed_point_op(
                    "div_pipe", width, int_width, frac_width, signed=is_signed
                ),
            )
        )

    return (
        init_cells
        + constants
        + product_registers
        + pow_registers
        + sum_registers
        + adds
        + pipes
        + reciprocal_factorials
        + pows
    )


def divide_and_conquer_sums(degree: int) -> List[Structure]:
    """Returns a list of groups for the sums.
    This is done by dividing the groups into
    log2(N) different rounds, where N is the `degree`.
    These rounds can then be executed in parallel.

    For example, with N == 4, we will produce groups:
      group sum_round1_1 { ... }     #    x   p2  p3  p4
                                     #     \  /   \  /
      group sum_round1_2 { ... }     #    sum1   sum2
                                     #       \   /
      group sum_round2_1 { ... }     #        sum1

      group add_degree_zero { ... }  #    sum1 + 1
    """
    groups = []
    sum_count = degree
    round = 1
    while sum_count > 1:
        indices = [i for i in range(1, sum_count + 1)]
        register_indices = [
            (lhs, rhs)
            for lhs, rhs in zip(
                list(filter(lambda x: (x % 2 != 0), indices)),
                list(filter(lambda x: (x % 2 == 0), indices)),
            )
        ]
        for i, (lhs, rhs) in enumerate(register_indices):
            group_name = CompVar(f"sum_round{round}_{i + 1}")
            adder = CompVar(f"add{i + 1}")

            # The first round will accrue its operands
            # from the previously calculated products.
            register_name = "product" if round == 1 else "sum"

            reg_lhs = CompVar(f"{register_name}{lhs}")
            reg_rhs = CompVar(f"{register_name}{rhs}")
            sum = CompVar(f"sum{i + 1}")

            # In the first round and first group, we add the 1st degree, the
            # value `x` itself.
            lhs = (
                CompPort(CompVar("frac_x"), "out")
                if round == 1 and i == 0
                else CompPort(reg_lhs, "out")
            )
            connections = [
                Connect(CompPort(adder, "left"), lhs),
                Connect(CompPort(adder, "right"), CompPort(reg_rhs, "out")),
                Connect(CompPort(sum, "write_en"), ConstantPort(1, 1)),
                Connect(CompPort(sum, "in"), CompPort(adder, "out")),
                Connect(HolePort(group_name, "done"), CompPort(sum, "done")),
            ]
            groups.append(Group(group_name, connections, 1))
        sum_count >>= 1
        round = round + 1

    # Sums the 0th degree value, 1, and the final
    # sum of the divide-and-conquer.
    group_name = CompVar("add_degree_zero")
    adder = CompVar("add1")
    reg = CompVar("sum1")
    groups.append(
        Group(
            id=group_name,
            connections=[
                Connect(CompPort(adder, "left"), CompPort(reg, "out")),
                Connect(
                    CompPort(adder, "right"),
                    CompPort(CompVar("one"), "out"),
                ),
                Connect(CompPort(reg, "write_en"), ConstantPort(1, 1)),
                Connect(CompPort(reg, "in"), CompPort(adder, "out")),
                Connect(HolePort(group_name, "done"), CompPort(reg, "done")),
            ],
            static_delay=1,
        )
    )
    return groups


def generate_groups(
    degree: int, width: int, int_width: int, is_signed: bool
) -> List[Structure]:
    frac_width = width - int_width

    input = CompVar("exponent_value")
    init = Group(
        id=CompVar("init"),
        connections=[
            Connect(CompPort(input, "write_en"), ConstantPort(1, 1)),
            Connect(CompPort(input, "in"), ThisPort(CompVar("x"))),
            Connect(HolePort(CompVar("init"), "done"),
                    CompPort(input, "done")),
        ],
        static_delay=1,
    )

    if is_signed:
        mult_pipe = CompVar("mult_pipe1")
        negate = Group(
            id=CompVar("negate"),
            connections=[
                Connect(CompPort(mult_pipe, "left"), CompPort(input, "out")),
                Connect(
                    CompPort(mult_pipe, "right"),
                    CompPort(CompVar("negative_one"), "out"),
                ),
                Connect(
                    CompPort(mult_pipe, "go"),
                    ConstantPort(1, 1),
                    Not(Atom(CompPort(mult_pipe, "done"))),
                ),
                Connect(CompPort(input, "write_en"),
                        CompPort(mult_pipe, "done")),
                Connect(CompPort(input, "in"), CompPort(mult_pipe, "out")),
                Connect(HolePort(CompVar("negate"), "done"),
                        CompPort(input, "done")),
            ],
        )

    # Initialization: split up the value `x` into its integer and fractional
    # values.
    split_bits = Group(
        id=CompVar("split_bits"),
        connections=[
            Connect(
                CompPort(CompVar("and0"), "left"),
                CompPort(CompVar("exponent_value"), "out"),
            ),
            Connect(
                CompPort(CompVar("and0"), "right"),
                ConstantPort(width, 2 ** width - 2 ** frac_width),
            ),
            Connect(
                CompPort(CompVar("rsh"), "left"),
                CompPort(CompVar("and0"), "out"),
            ),
            Connect(
                CompPort(CompVar("rsh"), "right"),
                ConstantPort(width, frac_width),
            ),
            Connect(
                CompPort(CompVar("and1"), "left"),
                CompPort(CompVar("exponent_value"), "out"),
            ),
            Connect(
                CompPort(CompVar("and1"), "right"),
                ConstantPort(width, (2 ** frac_width) - 1),
            ),
            Connect(
                CompPort(CompVar("int_x"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(
                CompPort(CompVar("frac_x"), "write_en"),
                ConstantPort(1, 1),
            ),
            Connect(
                CompPort(CompVar("int_x"), "in"),
                CompPort(CompVar("rsh"), "out"),
            ),
            Connect(
                CompPort(CompVar("frac_x"), "in"),
                CompPort(CompVar("and1"), "out"),
            ),
            Connect(
                HolePort(CompVar("split_bits"), "done"),
                ConstantPort(1, 1),
                And(
                    Atom(CompPort(CompVar("int_x"), "done")),
                    Atom(CompPort(CompVar("frac_x"), "done")),
                ),
            ),
        ],
    )

    def consume_pow(i: int) -> Group:
        # Write the output of pow{i} to register p{i}.
        reg = CompVar(f"p{i}")
        group_name = CompVar(f"consume_pow{i}")
        connections = [
            Connect(CompPort(reg, "write_en"), ConstantPort(1, 1)),
            Connect(CompPort(reg, "in"), CompPort(CompVar(f"pow{i}"), "out")),
            Connect(
                HolePort(group_name, "done"),
                ConstantPort(1, 1),
                CompPort(reg, "done"),
            ),
        ]
        return Group(group_name, connections, 1)

    def multiply_by_reciprocal_factorial(i: int) -> Group:
        # Multiply register p{i} with the reciprocal factorial.
        group_name = CompVar(f"mult_by_reciprocal_factorial{i}")
        mult_pipe = CompVar(f"mult_pipe{i}")
        reg = CompVar(f"p{i}")
        product = CompVar(f"product{i}")
        reciprocal = CompVar(f"reciprocal_factorial{i}")
        connections = [
            Connect(CompPort(mult_pipe, "left"), CompPort(reg, "out")),
            Connect(CompPort(mult_pipe, "right"), CompPort(reciprocal, "out")),
            Connect(
                CompPort(mult_pipe, "go"),
                ConstantPort(1, 1),
                Not(Atom(CompPort(mult_pipe, "done"))),
            ),
            Connect(CompPort(product, "write_en"),
                    CompPort(mult_pipe, "done")),
            Connect(CompPort(product, "in"), CompPort(mult_pipe, "out")),
            Connect(HolePort(group_name, "done"), CompPort(product, "done")),
        ]
        return Group(group_name, connections)

    def final_multiply(register_id: CompVar) -> List[Group]:
        # Multiply e^{fractional_value} * e^{integer_value},
        # and write it to register `m`.
        group_name = CompVar("final_multiply")
        mult_pipe = CompVar("mult_pipe1")
        reg = CompVar("m")
        return [
            Group(
                id=group_name,
                connections=[
                    Connect(
                        CompPort(mult_pipe, "left"),
                        CompPort(CompVar("pow1"), "out"),
                    ),
                    Connect(
                        CompPort(mult_pipe, "right"),
                        CompPort(CompVar("sum1"), "out"),
                    ),
                    Connect(
                        CompPort(mult_pipe, "go"),
                        ConstantPort(1, 1),
                        Not(Atom(CompPort(mult_pipe, "done"))),
                    ),
                    Connect(CompPort(reg, "write_en"),
                            CompPort(mult_pipe, "done")),
                    Connect(CompPort(reg, "in"), CompPort(mult_pipe, "out")),
                    Connect(HolePort(group_name, "done"),
                            CompPort(reg, "done")),
                ],
            )
        ]

    if is_signed:
        # Take the reciprocal, since the initial value was -x.
        div_pipe = CompVar("div_pipe")
        input = CompVar("m")
        reciprocal = Group(
            id=CompVar("reciprocal"),
            connections=[
                Connect(CompPort(div_pipe, "left"),
                        CompPort(CompVar("one"), "out")),
                Connect(CompPort(div_pipe, "right"), CompPort(input, "out")),
                Connect(
                    CompPort(div_pipe, "go"),
                    ConstantPort(1, 1),
                    Not(Atom(CompPort(div_pipe, "done"))),
                ),
                Connect(CompPort(input, "write_en"),
                        CompPort(div_pipe, "done")),
                Connect(CompPort(input, "in"),
                        CompPort(div_pipe, "out_quotient")),
                Connect(
                    HolePort(CompVar("reciprocal"), "done"),
                    CompPort(input, "done")
                ),
            ],
        )
        is_negative = CombGroup(
            id=CompVar("is_negative"),
            connections=[
                Connect(CompPort(CompVar("lt"), "left"),
                        ThisPort(CompVar("x"))),
                Connect(CompPort(CompVar("lt"), "right"),
                        ConstantPort(width, 0)),
            ]
        )

    # Connect final value to the `out` signal of the component.
    output_register = CompVar("m")
    out = [Connect(ThisPort(CompVar("out")), CompPort(output_register, "out"))]
    return (
        [init, split_bits]
        + ([negate, is_negative, reciprocal] if is_signed else [])
        + [consume_pow(j) for j in range(2, degree + 1)]
        + [multiply_by_reciprocal_factorial(k) for k in range(2, degree + 1)]
        + divide_and_conquer_sums(degree)
        + final_multiply(output_register)
        + out
    )


def generate_control(degree: int, is_signed: bool) -> Control:
    pow_invokes = [
        ParComp(
            [
                Invoke(
                    CompVar("pow1"),
                    [
                        ("base", CompPort(CompVar("e"), "out")),
                        ("integer_exp", CompPort(CompVar("int_x"), "out")),
                    ],
                    [],
                )
            ]
            + [
                Invoke(
                    CompVar(f"pow{i}"),
                    [
                        ("base", CompPort(CompVar("frac_x"), "out")),
                        ("integer_exp", CompPort(CompVar(f"c{i}"), "out")),
                    ],
                    [],
                )
                for i in range(2, degree + 1)
            ]
        )
    ]
    consume_pow = [
        ParComp([Enable(f"consume_pow{i}") for i in range(2, degree + 1)])]
    mult_by_reciprocal = [
        ParComp(
            [Enable(f"mult_by_reciprocal_factorial{i}")
             for i in range(2, degree + 1)]
        )
    ]

    divide_and_conquer = []
    Enable_count = degree >> 1
    for r in range(1, int(log2(degree) + 1)):
        divide_and_conquer.append(
            ParComp([Enable(f"sum_round{r}_{i}")
                    for i in range(1, Enable_count + 1)])
        )
        Enable_count >>= 1

    ending_sequence = [Enable("add_degree_zero"), Enable("final_multiply")] + (
        [
            If(
                CompPort(CompVar("lt"), "out"),
                CompVar("is_negative"),
                Enable("reciprocal"),
            )
        ]
        if is_signed
        else []
    )
    return SeqComp(
        [Enable("init")]
        + (
            [
                If(
                    CompPort(CompVar("lt"), "out"),
                    CompVar("is_negative"),
                    Enable("negate"),
                )
            ]
            if is_signed
            else []
        )
        + [Enable("split_bits")]
        + pow_invokes
        + consume_pow
        + mult_by_reciprocal
        + divide_and_conquer
        + ending_sequence
    )


def generate_exp_taylor_series_approximation(
    degree: int, width: int, int_width: int, is_signed: bool
) -> List[Component]:
    """Generates Calyx components to produce the Taylor series
    approximation of e^x to the provided degree. Given this is
    a Maclaurin series, it can be written more generally as:
        e^x = 1 + x + (x^2 / 2!) + (x^3 / 3!) + ... + (x^n / n!)
        where `n` is the nth degree.

    Let `i` be the integer value and `f` be the fractional value
    of `x`, so that `x = i + f`. We can then calculate `x` in
    the following manner:
        1. Compute `e^i` using `fp_pow`.
        2. Compute `e^f` using a Taylor series approximation.
        3. Since `e^x = e^(i+f)`, multiply `e^i * e^f`.

    Reference: https://en.wikipedia.org/wiki/Taylor_series#Exponential_function
    """
    # TODO(cgyurgyik): Support any degree.
    assert (
        degree > 0 and log2(degree).is_integer()
    ), f"The degree: {degree} should be a power of 2."
    return [
        Component(
            "exp",
            inputs=[PortDef(CompVar("x"), width)],
            outputs=[PortDef(CompVar("out"), width)],
            structs=generate_cells(degree, width, int_width, is_signed)
            + generate_groups(degree, width, int_width, is_signed),
            controls=generate_control(degree, is_signed),
        ),
        generate_fp_pow_component(width, int_width, is_signed),
    ]


if __name__ == "__main__":
    import argparse
    import json

    parser = argparse.ArgumentParser(
        description="`exp` using a Taylor Series approximation"
    )
    parser.add_argument("file", nargs="?", type=str)
    parser.add_argument("-d", "--degree", type=int)
    parser.add_argument("-w", "--width", type=int)
    parser.add_argument("-i", "--int_width", type=int)
    parser.add_argument("-s", "--is_signed", type=bool)

    args = parser.parse_args()

    degree, width, int_width, is_signed = None, None, None, None
    required_fields = [args.degree, args.width, args.int_width, args.is_signed]
    if all(map(lambda x: x is not None, required_fields)):
        degree = args.degree
        width = args.width
        int_width = args.int_width
        is_signed = args.is_signed
    elif args.file is not None:
        with open(args.file, "r") as f:
            spec = json.load(f)
            degree = spec["degree"]
            width = spec["width"]
            int_width = spec["int_width"]
            is_signed = spec["is_signed"]
    else:
        parser.error(
            "Need to pass either `-f FILE` or all of `-d DEGREE -w WIDTH -i INT_WIDTH`"
        )

    program = Program(
        imports=[
            Import("primitives/core.futil"),
            Import("primitives/binary_operators.futil")
        ],
        components=generate_exp_taylor_series_approximation(
            degree, width, int_width, is_signed
        ),
    )
    # Append a `main` component for testing purposes.
    program.components.append(
        Component(
            "main",
            inputs=[],
            outputs=[],
            structs=[
                Cell(CompVar("t"), Stdlib().register(width)),
                Cell(CompVar("x"), Stdlib().mem_d1(
                    width, 1, 1), is_external=True),
                Cell(
                    CompVar("ret"),
                    Stdlib().mem_d1(width, 1, 1),
                    is_external=True,
                ),
                Cell(CompVar("e"), CompInst("exp", [])),
                Group(
                    id=CompVar("init"),
                    connections=[
                        Connect(
                            CompPort(CompVar("x"), "addr0"),
                            ConstantPort(1, 0),
                        ),
                        Connect(
                            CompPort(CompVar("t"), "in"),
                            CompPort(CompVar("x"), "read_data"),
                        ),
                        Connect(
                            CompPort(CompVar("t"), "write_en"),
                            ConstantPort(1, 1),
                        ),
                        Connect(
                            HolePort(CompVar("init"), "done"),
                            CompPort(CompVar("t"), "done"),
                        ),
                    ],
                ),
                Group(
                    id=CompVar("write_to_memory"),
                    connections=[
                        Connect(
                            CompPort(CompVar("ret"), "addr0"),
                            ConstantPort(1, 0),
                        ),
                        Connect(
                            CompPort(CompVar("ret"), "write_en"),
                            ConstantPort(1, 1),
                        ),
                        Connect(
                            CompPort(CompVar("ret"), "write_data"),
                            CompPort(CompVar("e"), "out"),
                        ),
                        Connect(
                            HolePort(CompVar("write_to_memory"), "done"),
                            CompPort(CompVar("ret"), "done"),
                        ),
                    ],
                ),
            ],
            controls=SeqComp(
                [
                    Enable("init"),
                    Invoke(
                        id=CompVar("e"),
                        in_connects=[("x", CompPort(CompVar("t"), "out"))],
                        out_connects=[],
                    ),
                    Enable("write_to_memory"),
                ]
            ),
        )
    )
    program.emit()
