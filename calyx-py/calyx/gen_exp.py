from calyx import ast
from math import factorial, log2
from typing import List
from fud.stages.verilator import numeric_types


def generate_fp_pow_component(width: int, int_width: int) -> ast.Component:
    """Generates a fixed point `pow` component, which
    computes the value x**y, where y must be an integer.
    """
    stdlib = ast.Stdlib()
    frac_width = width - int_width

    pow = ast.CompVar("pow")
    count = ast.CompVar("count")
    mul = ast.CompVar("mul")
    lt = ast.CompVar("lt")
    incr = ast.CompVar("incr")

    cells = [
        ast.Cell(pow, stdlib.register(width)),
        ast.Cell(count, stdlib.register(width)),
        ast.Cell(
            mul,
            stdlib.fixed_point_op(
                "mult_pipe", width, int_width, frac_width, signed=False
            ),
        ),
        ast.Cell(lt, stdlib.op("lt", width, signed=False)),
        ast.Cell(incr, stdlib.op("add", width, signed=False)),
    ]
    wires = [
        ast.Group(
            id=ast.CompVar("init"),
            connections=[
                ast.Connect(
                    ast.ConstantPort(
                        width,
                        numeric_types.FixedPoint(
                            "1.0", width, int_width, is_signed=False
                        ).unsigned_integer(),
                    ),
                    ast.CompPort(pow, "in"),
                ),
                ast.Connect(ast.ConstantPort(1, 1), ast.CompPort(pow, "write_en")),
                ast.Connect(ast.ConstantPort(width, 0), ast.CompPort(count, "in")),
                ast.Connect(ast.ConstantPort(1, 1), ast.CompPort(count, "write_en")),
                ast.Connect(
                    ast.ConstantPort(1, 1),
                    ast.HolePort(ast.CompVar("init"), "done"),
                    ast.And(
                        ast.Atom(ast.CompPort(pow, "done")),
                        ast.Atom(ast.CompPort(count, "done")),
                    ),
                ),
            ],
        ),
        ast.Group(
            id=ast.CompVar("execute_mul"),
            connections=[
                ast.Connect(
                    ast.ThisPort(ast.CompVar("base")), ast.CompPort(mul, "left")
                ),
                ast.Connect(ast.CompPort(pow, "out"), ast.CompPort(mul, "right")),
                ast.Connect(
                    ast.ConstantPort(1, 1),
                    ast.CompPort(mul, "go"),
                    ast.Not(ast.Atom(ast.CompPort(mul, "done"))),
                ),
                ast.Connect(ast.CompPort(mul, "done"), ast.CompPort(pow, "write_en")),
                ast.Connect(ast.CompPort(mul, "out"), ast.CompPort(pow, "in")),
                ast.Connect(
                    ast.CompPort(pow, "done"),
                    ast.HolePort(ast.CompVar("execute_mul"), "done"),
                ),
            ],
        ),
        ast.Group(
            id=ast.CompVar("incr_count"),
            connections=[
                ast.Connect(ast.ConstantPort(width, 1), ast.CompPort(incr, "left")),
                ast.Connect(ast.CompPort(count, "out"), ast.CompPort(incr, "right")),
                ast.Connect(ast.CompPort(incr, "out"), ast.CompPort(count, "in")),
                ast.Connect(ast.ConstantPort(1, 1), ast.CompPort(count, "write_en")),
                ast.Connect(
                    ast.CompPort(count, "done"),
                    ast.HolePort(ast.CompVar("incr_count"), "done"),
                ),
            ],
        ),
        ast.Group(
            id=ast.CompVar("cond"),
            connections=[
                ast.Connect(ast.CompPort(count, "out"), ast.CompPort(lt, "left")),
                ast.Connect(
                    ast.ThisPort(ast.CompVar("integer_exp")), ast.CompPort(lt, "right")
                ),
                ast.Connect(
                    ast.ConstantPort(1, 1), ast.HolePort(ast.CompVar("cond"), "done")
                ),
            ],
        ),
        ast.Connect(
            ast.CompPort(ast.CompVar("pow"), "out"), ast.ThisPort(ast.CompVar("out"))
        ),
    ]
    return ast.Component(
        "fp_pow",
        inputs=[
            ast.PortDef(ast.CompVar("base"), width),
            ast.PortDef(ast.CompVar("integer_exp"), width),
        ],
        outputs=[ast.PortDef(ast.CompVar("out"), width)],
        structs=cells + wires,
        controls=ast.SeqComp(
            [
                ast.Enable("init"),
                ast.While(
                    ast.CompPort(lt, "out"),
                    ast.CompVar("cond"),
                    ast.ParComp([ast.Enable("execute_mul"), ast.Enable("incr_count")]),
                ),
            ]
        ),
    )


def float_to_fixed_point(value: float, N: int) -> float:
    """Returns a fixed point representation of `value`
    with the decimal value truncated to `N - 1` places.
    """
    w = 2 << (N - 1)
    return round(value * w) / float(w)


def generate_cells(degree: int, width: int, int_width: int) -> List[ast.Cell]:
    stdlib = ast.Stdlib()
    frac_width = width - int_width
    init_cells = [
        ast.Cell(ast.CompVar("int_x"), stdlib.register(width)),
        ast.Cell(ast.CompVar("frac_x"), stdlib.register(width)),
        ast.Cell(ast.CompVar("m"), stdlib.register(width)),
        ast.Cell(ast.CompVar("and0"), stdlib.op("and", width, signed=False)),
        ast.Cell(ast.CompVar("and1"), stdlib.op("and", width, signed=False)),
        ast.Cell(ast.CompVar("rsh"), stdlib.op("rsh", width, signed=False)),
    ]
    pow_registers = [
        ast.Cell(ast.CompVar(f"p{i}"), stdlib.register(width))
        for i in range(2, degree + 1)
    ]
    product_registers = [
        ast.Cell(ast.CompVar(f"product{i}"), stdlib.register(width))
        for i in range(2, degree + 1)
    ]
    sum_registers = [
        ast.Cell(ast.CompVar(f"sum{i}"), stdlib.register(width))
        for i in range(1, (degree // 2) + 1)
    ]
    adds = [
        ast.Cell(
            ast.CompVar(f"add{i}"),
            stdlib.fixed_point_op("add", width, int_width, frac_width, signed=False),
        )
        for i in range(1, (degree // 2) + 1)
    ]
    mult_pipes = [
        ast.Cell(
            ast.CompVar(f"mult_pipe{i}"),
            stdlib.fixed_point_op(
                "mult_pipe", width, int_width, frac_width, signed=False
            ),
        )
        for i in range(1, degree + 1)
    ]
    # One extra `fp_pow` instance to compute e^{int_value}.
    pows = [
        ast.Cell(ast.CompVar(f"pow{i}"), ast.CompInst("fp_pow", []))
        for i in range(1, degree + 1)
    ]
    reciprocal_factorials = []
    for i in range(2, degree + 1):
        fixed_point_value = float_to_fixed_point(1.0 / factorial(i), frac_width)
        value = numeric_types.FixedPoint(
            str(fixed_point_value), width, int_width, is_signed=False
        ).unsigned_integer()
        reciprocal_factorials.append(
            ast.Cell(
                ast.CompVar(f"reciprocal_factorial{i}"), stdlib.constant(width, value)
            )
        )
    # Constant values for the exponent to the fixed point `pow` function.
    constants = [
        ast.Cell(ast.CompVar(f"c{i}"), stdlib.constant(width, i))
        for i in range(2, degree + 1)
    ] + [
        ast.Cell(
            ast.CompVar("one"),
            stdlib.constant(
                width,
                numeric_types.FixedPoint(
                    "1.0", width, int_width, is_signed=False
                ).unsigned_integer(),
            ),
        ),
        ast.Cell(
            ast.CompVar("e"),
            stdlib.constant(
                width,
                numeric_types.FixedPoint(
                    str(float_to_fixed_point(2.7182818284, frac_width)),
                    width,
                    int_width,
                    is_signed=False,
                ).unsigned_integer(),
            ),
        ),
    ]
    return (
        init_cells
        + constants
        + product_registers
        + pow_registers
        + sum_registers
        + adds
        + mult_pipes
        + reciprocal_factorials
        + pows
    )


def divide_and_conquer_sums(degree: int) -> List[ast.Structure]:
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
            group_name = ast.CompVar(f"sum_round{round}_{i + 1}")
            adder = ast.CompVar(f"add{i + 1}")

            # The first round will accrue its operands
            # from the previously calculated products.
            register_name = "product" if round == 1 else "sum"

            reg_lhs = ast.CompVar(f"{register_name}{lhs}")
            reg_rhs = ast.CompVar(f"{register_name}{rhs}")
            sum = ast.CompVar(f"sum{i + 1}")

            # In the first round and first group, we add the 1st degree, the value `x` itself.
            lhs = (
                ast.CompPort(ast.CompVar("frac_x"), "out")
                if round == 1 and i == 0
                else ast.CompPort(reg_lhs, "out")
            )
            connections = [
                ast.Connect(lhs, ast.CompPort(adder, "left")),
                ast.Connect(ast.CompPort(reg_rhs, "out"), ast.CompPort(adder, "right")),
                ast.Connect(ast.ConstantPort(1, 1), ast.CompPort(sum, "write_en")),
                ast.Connect(ast.CompPort(adder, "out"), ast.CompPort(sum, "in")),
                ast.Connect(
                    ast.CompPort(sum, "done"), ast.HolePort(group_name, "done")
                ),
            ]
            groups.append(ast.Group(group_name, connections, 1))
        sum_count >>= 1
        round = round + 1

    # Sums the 0th degree value, 1, and the final
    # sum of the divide-and-conquer.
    group_name = ast.CompVar(f"add_degree_zero")
    adder = ast.CompVar("add1")
    reg = ast.CompVar("sum1")
    groups.append(
        ast.Group(
            id=group_name,
            connections=[
                ast.Connect(ast.CompPort(reg, "out"), ast.CompPort(adder, "left")),
                ast.Connect(
                    ast.CompPort(ast.CompVar("one"), "out"),
                    ast.CompPort(adder, "right"),
                ),
                ast.Connect(ast.ConstantPort(1, 1), ast.CompPort(reg, "write_en")),
                ast.Connect(ast.CompPort(adder, "out"), ast.CompPort(reg, "in")),
                ast.Connect(
                    ast.CompPort(reg, "done"), ast.HolePort(group_name, "done")
                ),
            ],
            static_delay=1,
        )
    )
    return groups


def generate_groups(degree: int, width: int, int_width: int) -> List[ast.Structure]:
    frac_width = width - int_width

    # Initialization: split up the value `x` into its integer and fractional values.
    init = [
        ast.Group(
            id=ast.CompVar("init"),
            connections=[
                ast.Connect(
                    ast.ThisPort(ast.CompVar("x")),
                    ast.CompPort(ast.CompVar("and0"), "left"),
                ),
                ast.Connect(
                    ast.ConstantPort(width, 2 ** width - 2 ** frac_width),
                    ast.CompPort(ast.CompVar("and0"), "right"),
                ),
                ast.Connect(
                    ast.CompPort(ast.CompVar("and0"), "out"),
                    ast.CompPort(ast.CompVar("rsh"), "left"),
                ),
                ast.Connect(
                    ast.ConstantPort(width, frac_width),
                    ast.CompPort(ast.CompVar("rsh"), "right"),
                ),
                ast.Connect(
                    ast.ThisPort(ast.CompVar("x")),
                    ast.CompPort(ast.CompVar("and1"), "left"),
                ),
                ast.Connect(
                    ast.ConstantPort(width, (2 ** frac_width) - 1),
                    ast.CompPort(ast.CompVar("and1"), "right"),
                ),
                ast.Connect(
                    ast.ConstantPort(1, 1),
                    ast.CompPort(ast.CompVar("int_x"), "write_en"),
                ),
                ast.Connect(
                    ast.ConstantPort(1, 1),
                    ast.CompPort(ast.CompVar("frac_x"), "write_en"),
                ),
                ast.Connect(
                    ast.CompPort(ast.CompVar("rsh"), "out"),
                    ast.CompPort(ast.CompVar("int_x"), "in"),
                ),
                ast.Connect(
                    ast.CompPort(ast.CompVar("and1"), "out"),
                    ast.CompPort(ast.CompVar("frac_x"), "in"),
                ),
                ast.Connect(
                    ast.ConstantPort(1, 1),
                    ast.HolePort(ast.CompVar("init"), "done"),
                    ast.And(
                        ast.Atom(ast.CompPort(ast.CompVar("int_x"), "done")),
                        ast.Atom(ast.CompPort(ast.CompVar("frac_x"), "done")),
                    ),
                ),
            ],
        )
    ]

    def consume_pow(i: int) -> ast.Group:
        # Write the output of pow{i} to register p{i}.
        reg = ast.CompVar(f"p{i}")
        group_name = ast.CompVar(f"consume_pow{i}")
        connections = [
            ast.Connect(ast.ConstantPort(1, 1), ast.CompPort(reg, "write_en")),
            ast.Connect(
                ast.CompPort(ast.CompVar(f"pow{i}"), "out"), ast.CompPort(reg, "in")
            ),
            ast.Connect(
                ast.ConstantPort(1, 1),
                ast.HolePort(group_name, "done"),
                ast.CompPort(reg, "done"),
            ),
        ]
        return ast.Group(group_name, connections, 1)

    def multiply_by_reciprocal_factorial(i: int) -> ast.Group:
        # Multiply register p{i} with the reciprocal factorial.
        group_name = ast.CompVar(f"mult_by_reciprocal_factorial{i}")
        mult_pipe = ast.CompVar(f"mult_pipe{i}")
        reg = ast.CompVar(f"p{i}")
        product = ast.CompVar(f"product{i}")
        reciprocal = ast.CompVar(f"reciprocal_factorial{i}")
        connections = [
            ast.Connect(ast.CompPort(reg, "out"), ast.CompPort(mult_pipe, "left")),
            ast.Connect(
                ast.CompPort(reciprocal, "out"), ast.CompPort(mult_pipe, "right")
            ),
            ast.Connect(
                ast.ConstantPort(1, 1),
                ast.CompPort(mult_pipe, "go"),
                ast.Not(ast.Atom(ast.CompPort(mult_pipe, "done"))),
            ),
            ast.Connect(
                ast.CompPort(mult_pipe, "done"), ast.CompPort(product, "write_en")
            ),
            ast.Connect(ast.CompPort(mult_pipe, "out"), ast.CompPort(product, "in")),
            ast.Connect(
                ast.CompPort(product, "done"), ast.HolePort(group_name, "done")
            ),
        ]
        return ast.Group(group_name, connections)

    def final_multiply():
        # Multiply e^fractional_value * e^integer_value.
        group_name = ast.CompVar("final_multiply")
        mult_pipe = ast.CompVar("mult_pipe1")
        reg = ast.CompVar("m")
        connections = [
            ast.Connect(
                ast.CompPort(ast.CompVar("pow1"), "out"),
                ast.CompPort(mult_pipe, "left"),
            ),
            ast.Connect(
                ast.CompPort(ast.CompVar("sum1"), "out"),
                ast.CompPort(mult_pipe, "right"),
            ),
            ast.Connect(
                ast.ConstantPort(1, 1),
                ast.CompPort(mult_pipe, "go"),
                ast.Not(ast.Atom(ast.CompPort(mult_pipe, "done"))),
            ),
            ast.Connect(ast.CompPort(mult_pipe, "done"), ast.CompPort(reg, "write_en")),
            ast.Connect(ast.CompPort(mult_pipe, "out"), ast.CompPort(reg, "in")),
            ast.Connect(ast.CompPort(reg, "done"), ast.HolePort(group_name, "done")),
        ]
        return [ast.Group(group_name, connections)]

    # ast.Connect final sum to the `out` signal of the component
    out = [
        ast.Connect(
            ast.CompPort(ast.CompVar("m"), "out"), ast.ThisPort(ast.CompVar("out"))
        )
    ]
    return (
        init
        + [consume_pow(j) for j in range(2, degree + 1)]
        + [multiply_by_reciprocal_factorial(k) for k in range(2, degree + 1)]
        + divide_and_conquer_sums(degree)
        + final_multiply()
        + out
    )


def generate_control(degree: int) -> ast.Control:
    pow_invokes = [
        ast.ParComp(
            [
                ast.Invoke(
                    ast.CompVar("pow1"),
                    [
                        ("base", ast.CompPort(ast.CompVar("e"), "out")),
                        ("integer_exp", ast.CompPort(ast.CompVar("int_x"), "out")),
                    ],
                    [],
                )
            ]
            + [
                ast.Invoke(
                    ast.CompVar(f"pow{i}"),
                    [
                        ("base", ast.CompPort(ast.CompVar("frac_x"), "out")),
                        ("integer_exp", ast.CompPort(ast.CompVar(f"c{i}"), "out")),
                    ],
                    [],
                )
                for i in range(2, degree + 1)
            ]
        )
    ]
    consume_pow = [
        ast.ParComp([ast.Enable(f"consume_pow{i}") for i in range(2, degree + 1)])
    ]
    mult_by_reciprocal = [
        ast.ParComp(
            [
                ast.Enable(f"mult_by_reciprocal_factorial{i}")
                for i in range(2, degree + 1)
            ]
        )
    ]

    divide_and_conquer = []
    ast.Enable_count = degree >> 1
    for r in range(1, int(log2(degree) + 1)):
        divide_and_conquer.append(
            ast.ParComp(
                [
                    ast.Enable(f"sum_round{r}_{i}")
                    for i in range(1, ast.Enable_count + 1)
                ]
            )
        )
        ast.Enable_count >>= 1

    return ast.SeqComp(
        [ast.Enable("init")]
        + pow_invokes
        + consume_pow
        + mult_by_reciprocal
        + divide_and_conquer
        + [ast.Enable("add_degree_zero")]
        + [ast.Enable("final_multiply")]
    )


# TODO(cgyurgyik): Support negative values.
# We can do this in the following manner:
#   if (x < 0.0): out = 1 / e^x
def generate_exp_taylor_series_approximation(
    degree: int, width: int, int_width: int
) -> ast.Program:
    """Generates a Calyx program to produce the Taylor Series
    approximation of e^x to the provided degree. Given this is
    a Maclaurin series, it can be written more generally as:
        e^x = 1 + x + (x^2 / 2!) + (x^3 / 3!) + ... + (x^n / n!)
        where `n` is the nth degree.

    Let `i` be the integer value and `f` be the fractional value
    of `x`, so that `x = i + f`. We can then calculate `x` in
    the following manner:
        1. Compute `e^i` using `fp_pow`.
        2. Compute `e^f` using a Taylor Series approximation.
        3. Since `e^x = e^(i+f)`, multiply `e^i * e^f`.

    Reference: https://en.wikipedia.org/wiki/Taylor_series#Exponential_function
    """
    # TODO(cgyurgyik): Support any degree.
    assert (
        degree > 0 and log2(degree).is_integer()
    ), f"The degree: {degree} should be a power of 2."
    return ast.Program(
        imports=[ast.Import("primitives/std.lib")],
        components=[
            ast.Component(
                "exp",
                inputs=[ast.PortDef(ast.CompVar("x"), width)],
                outputs=[ast.PortDef(ast.CompVar("out"), width)],
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
        description="`exp` using a Taylor Series approximation"
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
        ast.Component(
            "main",
            inputs=[],
            outputs=[],
            structs=[
                ast.Cell(ast.CompVar("t"), ast.Stdlib().register(width)),
                ast.Cell(
                    ast.CompVar("x"), ast.Stdlib().mem_d1(width, 1, 1), is_external=True
                ),
                ast.Cell(
                    ast.CompVar("ret"),
                    ast.Stdlib().mem_d1(width, 1, 1),
                    is_external=True,
                ),
                ast.Cell(ast.CompVar("e"), ast.CompInst("exp", [])),
                ast.Group(
                    id=ast.CompVar("init"),
                    connections=[
                        ast.Connect(
                            ast.ConstantPort(1, 0),
                            ast.CompPort(ast.CompVar("x"), "addr0"),
                        ),
                        ast.Connect(
                            ast.CompPort(ast.CompVar("x"), "read_data"),
                            ast.CompPort(ast.CompVar("t"), "in"),
                        ),
                        ast.Connect(
                            ast.ConstantPort(1, 1),
                            ast.CompPort(ast.CompVar("t"), "write_en"),
                        ),
                        ast.Connect(
                            ast.CompPort(ast.CompVar("t"), "done"),
                            ast.HolePort(ast.CompVar("init"), "done"),
                        ),
                    ],
                ),
                ast.Group(
                    id=ast.CompVar("write_to_memory"),
                    connections=[
                        ast.Connect(
                            ast.ConstantPort(1, 0),
                            ast.CompPort(ast.CompVar("ret"), "addr0"),
                        ),
                        ast.Connect(
                            ast.ConstantPort(1, 1),
                            ast.CompPort(ast.CompVar("ret"), "write_en"),
                        ),
                        ast.Connect(
                            ast.CompPort(ast.CompVar("e"), "out"),
                            ast.CompPort(ast.CompVar("ret"), "write_data"),
                        ),
                        ast.Connect(
                            ast.CompPort(ast.CompVar("ret"), "done"),
                            ast.HolePort(ast.CompVar("write_to_memory"), "done"),
                        ),
                    ],
                ),
            ],
            controls=ast.SeqComp(
                [
                    ast.Enable("init"),
                    ast.Invoke(
                        id=ast.CompVar("e"),
                        in_connects=[("x", ast.CompPort(ast.CompVar("t"), "out"))],
                        out_connects=[],
                    ),
                    ast.Enable("write_to_memory"),
                ]
            ),
        )
    )
    program.emit()
