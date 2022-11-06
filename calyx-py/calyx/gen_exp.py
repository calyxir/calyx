from typing import List
from calyx.py_ast import (
    Connect, CompVar, Cell, Group, ConstantPort, CompPort, Stdlib,
    Component, ThisPort, And, HolePort, Atom, Not, PortDef, SeqComp,
    Enable, While, ParComp, Structure, CompInst, Invoke, Program, Control,
    If, Import, CombGroup
)
from calyx.utils import float_to_fixed_point
from math import factorial, log2, log
from fud.stages.verilator import numeric_types
from calyx.gen_ln import generate_ln


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
            ]
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
                Connect(CompPort(CompVar(f"add{i + 1}"), "left"), lhs),
                Connect(CompPort(CompVar(f"add{i + 1}"),
                        "right"), CompPort(reg_rhs, "out")),
                Connect(CompPort(sum, "write_en"), ConstantPort(1, 1)),
                Connect(CompPort(sum, "in"), CompPort(CompVar(f"add{i + 1}"), "out")),
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


def consume_pow(i: int) -> Group:
    ''' Write the output of pow{i} to register p{i}. '''
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
    ''' Multiply register p{i} with the reciprocal factorial. '''
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


def generate_groups(
    degree: int, width: int, int_width: int, is_signed: bool
) -> List[Structure]:
    frac_width = width - int_width

    input = CompVar("exponent_value")

    init_exp = Group(
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
        ([init_exp, split_bits])
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
            ([
                Invoke(
                    CompVar("pow1"),
                    [
                        ("base", CompPort(CompVar("e"), "out")),
                        ("integer_exp", CompPort(CompVar("int_x"), "out")),
                    ],
                    [],
                )
            ])
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

    final_calculation = [Enable("add_degree_zero"), Enable(
        "final_multiply")]
    ending_sequence = final_calculation + (
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
        + ([Enable("split_bits")])
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


def gen_reciprocal(name, base_cell, div_pipe, const_one):
    '''
    Generates a group that takes in a base cell and sets its new value to its reciprocal
    '''
    return Group(
        id=CompVar(name),
        connections=[
            Connect(
                CompPort(div_pipe.id, "left"),
                CompPort(const_one.id, "out"),
            ),
            Connect(
                CompPort(div_pipe.id, "right"),
                CompPort(base_cell.id, "out"),
            ),
            Connect(
                CompPort(div_pipe.id, "go"),
                ConstantPort(1, 1),
                Not(Atom(CompPort(div_pipe.id, "done"))),
            ),
            Connect(
                CompPort(base_cell.id, "write_en"),
                CompPort(div_pipe.id, "done"),
            ),
            Connect(
                CompPort(base_cell.id, "in"),
                CompPort(div_pipe.id, "out_quotient"),
            ),
            Connect(
                HolePort(CompVar(name), "done"),
                CompPort(base_cell.id, "done"),
            )
        ],
    )


def gen_reverse_sign(name, base_cell, mult_pipe, const_neg_one):
    '''
    Generates a group that takes in a base cell and multiplies it by negative one
    '''
    return Group(
        id=CompVar(name),
        connections=[
            Connect(
                CompPort(mult_pipe.id, "left"),
                CompPort(base_cell.id, "out"),
            ),
            Connect(
                CompPort(mult_pipe.id, "right"),
                CompPort(const_neg_one.id, "out"),
            ),
            Connect(
                CompPort(mult_pipe.id, "go"),
                ConstantPort(1, 1),
                Not(Atom(CompPort(mult_pipe.id, "done"))),
            ),
            Connect(
                CompPort(base_cell.id, "write_en"),
                CompPort(mult_pipe.id, "done"),
            ),
            Connect(
                CompPort(base_cell.id, "in"),
                CompPort(mult_pipe.id, "out"),
            ),
            Connect(
                HolePort(CompVar(name), "done"),
                CompPort(base_cell.id, "done"),
            )
        ],
    )


def gen_comb_lt(name, lhs, lt, const_cell):
    '''
    Generates lhs < const_cell
    '''
    return CombGroup(
        id=CompVar(name),
        connections=[
            Connect(CompPort(lt.id, "left"), lhs),
            Connect(CompPort(lt.id, "right"), CompPort(const_cell.id, "out")),
        ],
    )


def gen_constant_cell(name, value, width, int_width, is_signed) -> Cell:
    stdlib = Stdlib()
    return Cell(
        CompVar(name),
        stdlib.constant(
            width,
            numeric_types.FixedPoint(
                value, width, int_width, is_signed=is_signed
            ).unsigned_integer(),
        ),
    )


def generate_fp_pow_full(
    degree: int, width: int, int_width: int, is_signed: bool
) -> List[Component]:
    '''
    Generates a component that can calculate b^x, for any fixed point b and x.
    Here is the idea behind how the component works:
    b^x = e^ln(b^x) = e ^ (x*ln(b)).
    Therefore, we can use our ln component to calculate ln(b) and then multiply 
    ln(b) by x. Then we raise that result to the e (using the taylor series approximation)
    and get our result.
    '''
    stdlib = Stdlib()
    lt = Cell(CompVar("lt"), Stdlib().op("lt", width, is_signed))
    div = Cell(CompVar("div_pipe"), Stdlib().fixed_point_op(
        "div_pipe", width, int_width, width-int_width, is_signed))
    const_one = Cell(
        CompVar("one"),
        stdlib.constant(
            width,
            numeric_types.FixedPoint(
                "1.0", width, int_width, is_signed=is_signed
            ).unsigned_integer(),
        ),
    )
    const_zero = Cell(
        CompVar("zero"),
        stdlib.constant(
            width,
            numeric_types.FixedPoint(
                "0.0", width, int_width, is_signed=is_signed
            ).unsigned_integer(),
        ),
    )
    mult = Cell(CompVar("mult"), Stdlib().fixed_point_op(
        "mult_pipe", width, int_width, width-int_width, is_signed))
    new_base_reg = Cell(CompVar("new_base"), Stdlib().register(width))
    stored_base_reg = Cell(CompVar("stored_base"), Stdlib().register(width))
    res = Cell(CompVar("res"), Stdlib().register(width))
    base_reciprocal = If(port=CompPort(lt.id, "out"), cond=CompVar("base_lt_one"),
                         true_branch=Enable("set_base_reciprocal"))
    base_rev = If(port=CompPort(lt.id, "out"), cond=CompVar("base_lt_zero"),
                  true_branch=Enable("rev_base_sign"))
    res_rev = If(port=CompPort(lt.id, "out"), cond=CompVar("base_lt_zero"),
                 true_branch=Enable("rev_res_sign"))
    res_reciprocal = If(port=CompPort(lt.id, "out"), cond=CompVar("base_lt_one"),
                        true_branch=Enable("set_res_reciprocal"))

    pre_process = SeqComp(
        [base_rev, Enable("store_old_reg_val"), base_reciprocal]) if is_signed else SeqComp([Enable("store_old_reg_val"), base_reciprocal])
    post_process = SeqComp([res_rev, res_reciprocal]
                           ) if is_signed else SeqComp([res_reciprocal])

    if is_signed:
        const_neg_one = Cell(
            CompVar("neg_one"),
            stdlib.constant(
                width,
                numeric_types.FixedPoint(
                    "-1.0", width, int_width, is_signed=is_signed
                ).unsigned_integer(),
            ),
        )
        rev_structs = [gen_reverse_sign("rev_base_sign",
                                        new_base_reg, mult, const_neg_one), gen_reverse_sign("rev_res_sign", res, mult, const_neg_one), gen_comb_lt("base_lt_zero", ThisPort(CompVar("base")), lt, const_zero), const_neg_one]

    return (generate_exp_taylor_series_approximation(
        degree, width, int_width, is_signed) +
        generate_ln(width, int_width, is_signed) +
        [Component(
            "fp_pow_full",
            inputs=[PortDef(CompVar("base"), width), PortDef(CompVar("exp"), width)],
            outputs=[PortDef(CompVar("out"), width)],
            structs=(rev_structs if is_signed else []) + [
                const_one,
                mult,
                new_base_reg,
                stored_base_reg,
                res,
                lt,
                div,
                const_zero,
                Cell(CompVar("new_exp_val"), Stdlib().register(width)),
                Cell(CompVar("e"), CompInst("exp", [])),
                Cell(CompVar("l"), CompInst("ln", [])),
                Group(
                    id=CompVar("set_new_exp"),
                    connections=[
                        Connect(
                            CompPort(CompVar("mult"), "left"),
                            CompPort(CompVar("l"), "out"),
                        ),
                        Connect(
                            CompPort(CompVar("mult"), "right"),
                            ThisPort(CompVar("exp")),
                        ),
                        Connect(
                            CompPort(CompVar("mult"), "go"),
                            ConstantPort(1, 1),
                            Not(Atom(CompPort(CompVar("mult"), "done"))),
                        ),
                        Connect(
                            CompPort(CompVar("new_exp_val"), "write_en"),
                            CompPort(CompVar("mult"), "done"),
                        ),
                        Connect(
                            CompPort(CompVar("new_exp_val"), "in"),
                            CompPort(CompVar("mult"), "out"),
                        ),
                        Connect(
                            HolePort(CompVar("set_new_exp"), "done"),
                            CompPort(CompVar("new_exp_val"), "done"),
                        )
                    ],
                ),
                Connect(ThisPort(CompVar("out")), CompPort(CompVar("res"), "out")),
                Group(
                    id=CompVar("write_to_base_reg"),
                    connections=[
                        Connect(
                            CompPort(new_base_reg.id, "write_en"),
                            ConstantPort(1, 1),
                        ),
                        Connect(
                            CompPort(new_base_reg.id, "in"),
                            ThisPort(CompVar("base"))
                        ),
                        Connect(HolePort(CompVar("write_to_base_reg"), "done"),
                                CompPort(new_base_reg.id, "done"))
                    ]
                ),
                Group(
                    id=CompVar("store_old_reg_val"),
                    connections=[
                        Connect(
                            CompPort(stored_base_reg.id, "write_en"),
                            ConstantPort(1, 1),
                        ),
                        Connect(
                            CompPort(stored_base_reg.id, "in"),
                            CompPort(new_base_reg.id, "out")
                        ),
                        Connect(HolePort(CompVar("store_old_reg_val"), "done"),
                                CompPort(stored_base_reg.id, "done"))
                    ]
                ),
                Group(
                    id=CompVar("write_e_to_res"),
                    connections=[
                        Connect(
                            CompPort(res.id, "write_en"),
                            ConstantPort(1, 1),
                        ),
                        Connect(
                            CompPort(res.id, "in"),
                            CompPort(CompVar("e"), "out")
                        ),
                        Connect(HolePort(CompVar("write_e_to_res"), "done"),
                                CompPort(res.id, "done"))
                    ]
                ),
                gen_reciprocal("set_base_reciprocal", new_base_reg, div, const_one),
                gen_reciprocal("set_res_reciprocal", res, div, const_one),
                gen_comb_lt("base_lt_one", CompPort(
                    stored_base_reg.id, "out"), lt, const_one),
            ],
            controls=SeqComp(
                [
                    Enable("write_to_base_reg"),
                    pre_process,
                    Invoke(id=CompVar("l"),
                           in_connects=[("x", CompPort(new_base_reg.id, "out"))],
                           out_connects=[]),
                    Enable("set_new_exp"),
                    Invoke(
                        id=CompVar("e"),
                        in_connects=[("x", CompPort(CompVar("new_exp_val"), "out"))],
                        out_connects=[],
                    ),
                    Enable("write_e_to_res"),
                    post_process
                ]
            ),
        )])


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
    parser.add_argument("-e", "--base_is_e", type=bool)

    args = parser.parse_args()

    degree, width, int_width, is_signed, base_is_e = None, None, None, None, None
    required_fields = [args.degree, args.width,
                       args.int_width, args.is_signed, args.base_is_e]
    if all(map(lambda x: x is not None, required_fields)):
        degree = args.degree
        width = args.width
        int_width = args.int_width
        is_signed = args.is_signed
        base_is_e = args.base_is_e
    elif args.file is not None:
        with open(args.file, "r") as f:
            spec = json.load(f)
            degree = spec["degree"]
            width = spec["width"]
            int_width = spec["int_width"]
            is_signed = spec["is_signed"]
            base_is_e = spec["base_is_e"]
    else:
        parser.error(
            "Need to pass either `-f FILE` or all of `-d DEGREE -w WIDTH -i INT_WIDTH`"
        )

    # build 2 separate programs: 1 if base_is_e is true, the other if false
    # any_base_program is (obviously) the one for any base
    any_base_program = Program(
        imports=[Import("primitives/core.futil"),
                 Import("primitives/binary_operators.futil")],
        components=generate_fp_pow_full(degree, width, int_width, is_signed)
    )
    # main component for testing purposes
    any_base_main = Component(
        "main",
        inputs=[],
        outputs=[],
        structs=[
            Cell(CompVar("base_reg"), Stdlib().register(width)),
            Cell(CompVar("exp_reg"), Stdlib().register(width)),
            Cell(CompVar("x"), Stdlib().mem_d1(
                width, 1, 1), is_external=True),
            Cell(CompVar("b"), Stdlib().mem_d1(
                width, 1, 1), is_external=True),
            Cell(
                CompVar("ret"),
                Stdlib().mem_d1(width, 1, 1),
                is_external=True,
            ),
            Cell(CompVar("f"), CompInst("fp_pow_full", [])),
            Group(
                id=CompVar("read_base"),
                connections=[
                    Connect(
                        CompPort(CompVar("b"), "addr0"),
                        ConstantPort(1, 0),
                    ),
                    Connect(
                        CompPort(CompVar("base_reg"), "in"),
                        CompPort(CompVar("b"), "read_data"),
                    ),
                    Connect(
                        CompPort(CompVar("base_reg"), "write_en"),
                        ConstantPort(1, 1),
                    ),
                    Connect(
                        HolePort(CompVar("read_base"), "done"),
                        CompPort(CompVar("base_reg"), "done"),
                    ),
                ],
            ),
            Group(
                id=CompVar("read_exp"),
                connections=[
                    Connect(
                        CompPort(CompVar("x"), "addr0"),
                        ConstantPort(1, 0),
                    ),
                    Connect(
                        CompPort(CompVar("exp_reg"), "in"),
                        CompPort(CompVar("x"), "read_data"),
                    ),
                    Connect(
                        CompPort(CompVar("exp_reg"), "write_en"),
                        ConstantPort(1, 1),
                    ),
                    Connect(
                        HolePort(CompVar("read_exp"), "done"),
                        CompPort(CompVar("exp_reg"), "done"),
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
                        CompPort(CompVar("f"), "out"),
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
                Enable("read_base"),
                Enable("read_exp"),
                Invoke(
                    id=CompVar("f"),
                    in_connects=[("base", CompPort(CompVar("base_reg"), "out")),
                                 ("exp", CompPort(CompVar("exp_reg"), "out"))],
                    out_connects=[],
                ),
                Enable("write_to_memory"),
            ]
        ),
    )
    any_base_program.components.append(any_base_main)

    # this is the program for when the base = e
    program = Program(
        imports=[
            Import("primitives/core.futil"),
            Import("primitives/binary_operators.futil")
        ],
        components=generate_exp_taylor_series_approximation(
            degree, width, int_width, is_signed),
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

    if base_is_e:
        program.emit()
    else:
        any_base_program.emit()
