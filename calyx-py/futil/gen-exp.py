import numpy as np
from ast import *
from math import factorial, log2
from typing import List
from fud.stages.verilator.numeric_types import FixedPoint


def generate_fp_pow_component(width: int, int_width: int) -> Component:
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
                "mult_pipe", width, int_width, frac_width, signed=False
            ),
        ),
        Cell(lt, stdlib.op("lt", width, signed=False)),
        Cell(incr, stdlib.op("add", width, signed=False)),
    ]
    wires = [
        Group(
            id=CompVar("init"),
            connections=[
                Connect(
                    ConstantPort(
                        width,
                        FixedPoint(
                            "1.0", width, int_width, is_signed=False
                        ).unsigned_integer(),
                    ),
                    CompPort(pow, "in"),
                ),
                Connect(ConstantPort(1, 1), CompPort(pow, "write_en")),
                Connect(ConstantPort(width, 0), CompPort(count, "in")),
                Connect(ConstantPort(1, 1), CompPort(count, "write_en")),
                Connect(
                    ConstantPort(1, 1),
                    HolePort(CompVar("init"), "done"),
                    And(Atom(CompPort(pow, "done")), Atom(CompPort(count, "done"))),
                ),
            ],
        ),
        Group(
            id=CompVar("execute_mul"),
            connections=[
                Connect(ThisPort(CompVar("base")), CompPort(mul, "left")),
                Connect(CompPort(pow, "out"), CompPort(mul, "right")),
                Connect(
                    ConstantPort(1, 1),
                    CompPort(mul, "go"),
                    Not(Atom(CompPort(mul, "done"))),
                ),
                Connect(CompPort(mul, "done"), CompPort(pow, "write_en")),
                Connect(CompPort(mul, "out"), CompPort(pow, "in")),
                Connect(
                    CompPort(pow, "done"), HolePort(CompVar("execute_mul"), "done")
                ),
            ],
        ),
        Group(
            id=CompVar("incr_count"),
            connections=[
                Connect(ConstantPort(width, 1), CompPort(incr, "left")),
                Connect(CompPort(count, "out"), CompPort(incr, "right")),
                Connect(CompPort(incr, "out"), CompPort(count, "in")),
                Connect(ConstantPort(1, 1), CompPort(count, "write_en")),
                Connect(
                    CompPort(count, "done"), HolePort(CompVar("incr_count"), "done")
                ),
            ],
        ),
        Group(
            id=CompVar("cond"),
            connections=[
                Connect(CompPort(count, "out"), CompPort(lt, "left")),
                Connect(ThisPort(CompVar("integer_exp")), CompPort(lt, "right")),
                Connect(ConstantPort(1, 1), HolePort(CompVar("cond"), "done")),
            ],
        ),
        Connect(CompPort(CompVar("pow"), "out"), ThisPort(CompVar("out"))),
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


def float_to_fixed_point(value: float, N: int) -> float:
    """Returns a fixed point representation of `value`
    with the decimal value truncated to `N - 1` places.
    """
    w = 2 << (N - 1)
    return np.round(value * w) / float(w)


def generate_cells(degree: int, width: int, int_width: int) -> List[Cell]:
    stdlib = Stdlib()
    frac_width = width - int_width
    input_registers = [
        Cell(CompVar(f"x{i}"), stdlib.register(width)) for i in range(2, degree + 1)
    ]
    pow_registers = [
        Cell(CompVar(f"p{i}"), stdlib.register(width)) for i in range(2, degree + 1)
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
            stdlib.fixed_point_op("add", width, int_width, frac_width, signed=False),
        )
        for i in range(1, (degree // 2) + 1)
    ]
    mult_pipes = [
        Cell(
            CompVar(f"mult_pipe{i}"),
            stdlib.fixed_point_op(
                "mult_pipe", width, int_width, frac_width, signed=False
            ),
        )
        for i in range(2, degree + 1)
    ]
    pows = [
        Cell(CompVar(f"pow{i}"), CompInst("fp_pow", [])) for i in range(2, degree + 1)
    ]
    reciprocal_factorials = []
    for i in range(2, degree + 1):
        fixed_point_value = float_to_fixed_point(1.0 / factorial(i), frac_width)
        value = FixedPoint(
            str(fixed_point_value), width, int_width, is_signed=False
        ).unsigned_integer()
        reciprocal_factorials.append(
            Cell(CompVar(f"reciprocal_factorial{i}"), stdlib.constant(width, value))
        )
    # Constant values for the exponent to the fixed point `pow` function.
    constants = [
        Cell(CompVar(f"c{i}"), stdlib.constant(width, i)) for i in range(2, degree + 1)
    ]
    one = [
        Cell(
            CompVar(f"one"),
            stdlib.constant(
                width,
                FixedPoint("1.0", width, int_width, is_signed=False).unsigned_integer(),
            ),
        )
    ]
    return (
        constants
        + input_registers
        + product_registers
        + pow_registers
        + sum_registers
        + adds
        + mult_pipes
        + reciprocal_factorials
        + one
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
      out = sum1.out                 #    Hook to `out`

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

            # In the first round and first group, we add the 1st degree, the value `x` itself.
            lhs = (
                ThisPort(CompVar("x"))
                if round == 1 and i == 0
                else CompPort(reg_lhs, "out")
            )
            connections = [
                Connect(lhs, CompPort(adder, "left")),
                Connect(CompPort(reg_rhs, "out"), CompPort(adder, "right")),
                Connect(ConstantPort(1, 1), CompPort(sum, "write_en")),
                Connect(CompPort(adder, "out"), CompPort(sum, "in")),
                Connect(CompPort(sum, "done"), HolePort(group_name, "done")),
            ]
            groups.append(Group(group_name, connections, 1))
        sum_count >>= 1
        round = round + 1

    # Sums the 0th degree value, 1, and the final
    # sum of the divide-and-conquer.
    group_name = CompVar(f"add_degree_zero")
    adder = CompVar("add1")
    reg = CompVar("sum1")
    groups.append(
        Group(
            id=group_name,
            connections=[
                Connect(CompPort(reg, "out"), CompPort(adder, "left")),
                Connect(CompPort(CompVar("one"), "out"), CompPort(adder, "right")),
                Connect(ConstantPort(1, 1), CompPort(reg, "write_en")),
                Connect(CompPort(adder, "out"), CompPort(reg, "in")),
                Connect(CompPort(reg, "done"), HolePort(group_name, "done")),
            ],
            static_delay=1,
        )
    )

    # Connect final sum to the `out` signal of the component
    out = [Connect(CompPort(CompVar("sum1"), "out"), ThisPort(CompVar("out")))]
    return groups + out


def generate_groups(degree: int, width: int, int_width: int) -> List[Structure]:
    frac_width = width - int_width

    def assign_input(i: int) -> Group:
        # Assign value of the input `x`
        # to each temporary register x{i}.
        reg = CompVar(f"x{i}")
        group_name = CompVar(f"assign_input{i}")
        connections = [
            Connect(ConstantPort(1, 1), CompPort(reg, "write_en")),
            Connect(ThisPort(CompVar("x")), CompPort(reg, "in")),
            Connect(
                ConstantPort(1, 1), HolePort(group_name, "done"), CompPort(reg, "done")
            ),
        ]
        return Group(group_name, connections, 1)

    def consume_pow(i: int) -> Group:
        # Write the output of pow{i} to register p{i}.
        reg = CompVar(f"p{i}")
        group_name = CompVar(f"consume_pow{i}")
        connections = [
            Connect(ConstantPort(1, 1), CompPort(reg, "write_en")),
            Connect(CompPort(CompVar(f"pow{i}"), "out"), CompPort(reg, "in")),
            Connect(
                ConstantPort(1, 1), HolePort(group_name, "done"), CompPort(reg, "done")
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
            Connect(CompPort(reg, "out"), CompPort(mult_pipe, "left")),
            Connect(CompPort(reciprocal, "out"), CompPort(mult_pipe, "right")),
            Connect(
                ConstantPort(1, 1),
                CompPort(mult_pipe, "go"),
                Not(Atom(CompPort(mult_pipe, "done"))),
            ),
            Connect(CompPort(mult_pipe, "done"), CompPort(product, "write_en")),
            Connect(CompPort(mult_pipe, "out"), CompPort(product, "in")),
            Connect(CompPort(product, "done"), HolePort(group_name, "done")),
        ]
        return Group(group_name, connections)

    return (
        [assign_input(i) for i in range(2, degree + 1)]
        + [consume_pow(j) for j in range(2, degree + 1)]
        + [multiply_by_reciprocal_factorial(k) for k in range(2, degree + 1)]
        + divide_and_conquer_sums(degree)
    )


def generate_control(degree: int) -> Control:
    assigns = [ParComp([Enable(f"assign_input{i}") for i in range(2, degree + 1)])]
    pow_invokes = [
        ParComp(
            [
                Invoke(
                    CompVar(f"pow{i}"),
                    [
                        ("base", CompPort(CompVar(f"x{i}"), "out")),
                        ("integer_exp", CompPort(CompVar(f"c{i}"), "out")),
                    ],
                    [],
                )
                for i in range(2, degree + 1)
            ]
        )
    ]
    consume_pow = [ParComp([Enable(f"consume_pow{i}") for i in range(2, degree + 1)])]
    mult_by_reciprocal = [
        ParComp(
            [Enable(f"mult_by_reciprocal_factorial{i}") for i in range(2, degree + 1)]
        )
    ]

    divide_and_conquer = []
    enable_count = degree >> 1
    for r in range(1, int(log2(degree) + 1)):
        divide_and_conquer.append(
            ParComp([Enable(f"sum_round{r}_{i}") for i in range(1, enable_count + 1)])
        )
        enable_count >>= 1

    return SeqComp(
        assigns
        + pow_invokes
        + consume_pow
        + mult_by_reciprocal
        + divide_and_conquer
        + [Enable("add_degree_zero")]
    )


def generate_exp_taylor_series_approximation(
    degree: int, width: int, int_width: int
) -> Program:
    """Generates a Calyx program to produce the Taylor Series
    approximation of e^x to the provided degree. Given this is
    a Maclaurin series, it can be written more generally as:
        e^x = 1 + x + (x^2 / 2!) + (x^3 / 3!) + ... + (x^n / n!)
        where `n` is the nth degree.
    Reference: https://en.wikipedia.org/wiki/Taylor_series#Exponential_function
    """
    # TODO(cgyurgyik): Support any degree.
    assert (
        degree > 0 and log2(degree).is_integer()
    ), f"The degree: {degree} should be a power of 2."
    return Program(
        imports=[Import("primitives/std.lib")],
        components=[
            Component(
                "exp",
                inputs=[PortDef(CompVar("x"), width)],
                outputs=[PortDef(CompVar("out"), width)],
                structs=generate_cells(degree, width, int_width)
                + generate_groups(degree, width, int_width),
                controls=generate_control(degree),
            ),
            generate_fp_pow_component(width, int_width),
        ],
    )


if __name__ == "__main__":
    import argparse, json

    parser = argparse.ArgumentParser(
        description="`exp` using Taylor Series approximation"
    )
    parser.add_argument("file", nargs="?", type=str)
    parser.add_argument("-d", "--degree", type=int)
    parser.add_argument("-w", "--width", type=int)
    parser.add_argument("-i", "--int_width", type=int)

    args = parser.parse_args()

    degree, width, int_width = None, None, None
    required_fields = [args.degree, args.width, args.int_width]
    if all(map(lambda x: x is not None, required_fields)):
        degree = args.degree
        width = args.width
        int_width = args.int_width
    elif args.file is not None:
        with open(args.file, "r") as f:
            spec = json.load(f)
            degree = spec["degree"]
            width = spec["width"]
            int_width = spec["int_width"]
    else:
        parser.error(
            "Need to pass either `-f FILE` or all of `-d DEGREE -w WIDTH -i INT_WIDTH`"
        )
    program = generate_exp_taylor_series_approximation(degree, width, int_width)
    program.components.append(
        Component(
            "main",
            inputs=[],
            outputs=[],
            structs=[
                Cell(CompVar("t"), Stdlib().register(width)),
                Cell(CompVar("x"), Stdlib().mem_d1(width, 1, 1), is_external=True),
                Cell(CompVar("ret"), Stdlib().mem_d1(width, 1, 1), is_external=True),
                Cell(CompVar("e"), CompInst("exp", [])),
                Group(
                    id=CompVar("init"),
                    connections=[
                        Connect(ConstantPort(1, 0), CompPort(CompVar("x"), "addr0")),
                        Connect(
                            CompPort(CompVar("x"), "read_data"),
                            CompPort(CompVar("t"), "in"),
                        ),
                        Connect(ConstantPort(1, 1), CompPort(CompVar("t"), "write_en")),
                        Connect(
                            CompPort(CompVar("t"), "done"),
                            HolePort(CompVar("init"), "done"),
                        ),
                    ],
                ),
                Group(
                    id=CompVar("write_to_memory"),
                    connections=[
                        Connect(ConstantPort(1, 0), CompPort(CompVar("ret"), "addr0")),
                        Connect(
                            ConstantPort(1, 1), CompPort(CompVar("ret"), "write_en")
                        ),
                        Connect(
                            CompPort(CompVar("e"), "out"),
                            CompPort(CompVar("ret"), "write_data"),
                        ),
                        Connect(
                            CompPort(CompVar("ret"), "done"),
                            HolePort(CompVar("write_to_memory"), "done"),
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
