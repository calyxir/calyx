from typing import List

from calyx.py_ast import (
    Connect,
    CompVar,
    Cell,
    Group,
    ConstantPort,
    CompPort,
    Stdlib,
    Component,
    ThisPort,
    And,
    HolePort,
    Atom,
    Not,
    PortDef,
    SeqComp,
    Enable,
    ParComp,
    Structure,
    CompInst,
    Invoke,
    Program,
    Control,
    If,
    Import,
    CombGroup,
)
from calyx.utils import float_to_fixed_point
from math import factorial, log2
from fud.stages.verilator import numeric_types
from calyx.gen_ln import generate_ln

from calyx.builder import (
    Builder,
    ComponentBuilder,
    while_,
    if_,
    invoke,
    CellBuilder,
    ExprBuilder,
)


def generate_fp_pow_component(
    builder: Builder, width: int, int_width: int, is_signed: bool
) -> Component:
    """Generates a fixed point `pow` component, which
    computes the value x**y, where y must be an integer.
    """
    frac_width = width - int_width

    # Component sigs
    comp = builder.component(name="fp_pow")
    comp.input("base", width)
    comp.input("integer_exp", width)
    comp.output("out", width)

    # cells
    pow = comp.reg("pow", width)
    count = comp.reg("count", width)
    mul = comp.cell(
        "mul",
        Stdlib.fixed_point_op(
            "mult_pipe", width, int_width, frac_width, signed=is_signed
        ),
    )
    lt = comp.cell("lt", Stdlib.op("lt", width, signed=is_signed))
    incr = comp.cell("incr", Stdlib.op("add", width, signed=is_signed))

    # groups
    with comp.group("init") as init:
        pow.in_ = numeric_types.FixedPoint(
            "1.0", width, int_width, is_signed=is_signed
        ).unsigned_integer()
        pow.write_en = 1
        count.in_ = 0
        count.write_en = 1
        init.done = (pow.done & count.done) @ 1

    with comp.group("execute_mul") as execute_mul:
        mul.left = comp.this().base
        mul.right = pow.out
        mul.go = (~mul.done) @ 1
        pow.write_en = mul.done
        pow.in_ = mul.out
        execute_mul.done = pow.done

    with comp.group("incr_count") as incr_count:
        incr.left = 1
        incr.right = count.out
        count.in_ = incr.out
        count.write_en = 1
        incr_count.done = count.done

    with comp.comb_group("cond") as cond:
        lt.left = count.out
        lt.right = comp.this().integer_exp

    with comp.continuous:
        comp.this().out = pow.out

    comp.control += [init, while_(lt.out, cond, {execute_mul, incr_count})]

    return comp.component


def generate_cells(
    comp: ComponentBuilder, degree: int, width: int, int_width: int, is_signed: bool
):
    frac_width = width - int_width

    # Init Cells
    comp.reg("exponent_value", width)
    comp.reg("int_x", width)
    comp.reg("frac_x", width)
    comp.reg("m", width)
    comp.cell("and0", Stdlib.op("and", width, signed=False))
    comp.cell("and1", Stdlib.op("and", width, signed=False))
    comp.cell("rsh", Stdlib.op("rsh", width, signed=False))
    if is_signed:
        comp.cell("lt", Stdlib.op("lt", width, signed=is_signed))

    # constants
    for i in range(2, degree + 1):
        comp.const(f"c{i}", width, i)

    # Constant values for the exponent to the fixed point `pow` function.
    comp.const(
        "one",
        width,
        numeric_types.FixedPoint(
            "1.0", width, int_width, is_signed=is_signed
        ).unsigned_integer(),
    )
    comp.const(
        "e",
        width,
        numeric_types.FixedPoint(
            str(float_to_fixed_point(2.7182818284, frac_width)),
            width,
            int_width,
            is_signed=is_signed,
        ).unsigned_integer(),
    )

    if is_signed:
        comp.const(
            "negative_one",
            width,
            numeric_types.FixedPoint(
                "-1.0", width, int_width, is_signed=is_signed
            ).unsigned_integer(),
        )

    # product and pow registers
    for i in range(2, degree + 1):
        comp.reg(f"product{i}", width)

    for i in range(2, degree + 1):
        comp.reg(f"p{i}", width)

    # sum registers and adders
    for i in range(1, (degree // 2) + 1):
        comp.reg(f"sum{i}", width)

    for i in range(1, (degree // 2) + 1):
        comp.cell(
            f"add{i}",
            Stdlib.fixed_point_op(
                "add", width, int_width, frac_width, signed=is_signed
            ),
        )

    # mult pipes
    for i in range(1, degree + 1):
        comp.cell(
            f"mult_pipe{i}",
            Stdlib.fixed_point_op(
                "mult_pipe", width, int_width, frac_width, signed=is_signed
            ),
        )

    if is_signed:
        comp.cell(
            "div_pipe",
            Stdlib.fixed_point_op(
                "div_pipe", width, int_width, frac_width, signed=is_signed
            ),
        )

    # reciprocal factorials
    for i in range(2, degree + 1):
        fixed_point_value = float_to_fixed_point(1.0 / factorial(i), frac_width)
        value = numeric_types.FixedPoint(
            str(fixed_point_value), width, int_width, is_signed=is_signed
        ).unsigned_integer()
        comp.const(f"reciprocal_factorial{i}", width, value)

    # One extra `fp_pow` instance to compute e^{int_value}.
    for i in range(1, degree + 1):
        comp.cell(f"pow{i}", CompInst("fp_pow", []))


def divide_and_conquer_sums(comp: ComponentBuilder, degree: int):
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
            register_name = "product" if round == 1 else "sum"

            # TODO (griffin): double check that the cells are being created
            # somewhere
            sum = comp.get_cell(f"sum{i + 1}")
            # The first round will accrue its operands
            # from the previously calculated products.

            reg_lhs = comp.get_cell(f"{register_name}{lhs}")
            reg_rhs = comp.get_cell(f"{register_name}{rhs}")
            adder = comp.get_cell(f"add{i + 1}")
            frac_x = comp.get_cell("frac_x")

            with comp.group(f"sum_round{round}_{i + 1}", static_delay=1) as grp:
                # In the first round and first group, we add the 1st degree, the
                # value `x` itself.
                if round == 1 and i == 0:
                    adder.left = frac_x.out
                else:
                    adder.left = reg_lhs.out
                adder.right = reg_rhs.out

                sum.write_en = 1
                sum.in_ = adder.out
                grp.done = sum.done

        sum_count >>= 1
        round = round + 1

    # Sums the 0th degree value, 1, and the final
    # sum of the divide-and-conquer.

    adder = comp.get_cell("add1")
    reg = comp.get_cell("sum1")
    one = comp.get_cell("one")

    with comp.group("add_degree_zero", static_delay=1) as grp:
        adder.left = reg.out
        adder.right = one.out
        reg.write_en = 1
        reg.in_ = adder.out
        grp.done = reg.done


def consume_pow(comp: ComponentBuilder, i: int):
    """Write the output of pow{i} to register p{i}."""
    reg = comp.get_cell(f"p{i}")
    pow = comp.get_cell(f"pow{i}")
    with comp.group(f"consume_pow{i}", static_delay=1) as grp:
        reg.write_en = 1
        reg.in_ = pow.out
        grp.done = reg.done @ 1


def multiply_by_reciprocal_factorial(comp: ComponentBuilder, i: int):
    """Multiply register p{i} with the reciprocal factorial."""
    mult_pipe = comp.get_cell(f"mult_pipe{i}")
    reg = comp.get_cell(f"p{i}")
    product = comp.get_cell(f"product{i}")
    reciprocal = comp.get_cell(f"reciprocal_factorial{i}")
    with comp.group(f"mult_by_reciprocal_factorial{i}") as grp:
        mult_pipe.left = reg.out
        mult_pipe.right = reciprocal.out
        mult_pipe.go = (~mult_pipe.done) @ 1
        product.write_en = mult_pipe.done
        product.in_ = mult_pipe.out
        grp.done = product.done


def final_multiply(comp: ComponentBuilder, register_id: CompVar):
    # Multiply e^{fractional_value} * e^{integer_value},
    # and write it to register `m`.

    # TODO (griffin): register_id is not used anywhere, I've matched the
    # original code in this translation but this should be fixed. Probably
    # removed given that the value is always supposed to be "m".

    mult_pipe = comp.get_cell("mult_pipe1")
    reg = comp.get_cell("m")
    sum = comp.get_cell("sum1")
    pow = comp.get_cell("pow1")

    with comp.group("final_multiply") as grp:
        mult_pipe.left = pow.out
        mult_pipe.right = sum.out
        mult_pipe.go = (~mult_pipe.done) @ 1
        reg.write_en = mult_pipe.done
        reg.in_ = mult_pipe.out
        grp.done = reg.done


def generate_groups(
    comp: ComponentBuilder, degree: int, width: int, int_width: int, is_signed: bool
):
    frac_width = width - int_width

    input = comp.get_cell("exponent_value")
    with comp.group("init", static_delay=1) as init:
        input.write_en = 1
        input.in_ = comp.this().x
        init.done = input.done

    # Initialization: split up the value `x` into its integer and fractional
    # values.
    and0 = comp.get_cell("and0")
    rsh = comp.get_cell("rsh")
    and1 = comp.get_cell("and1")
    int_x = comp.get_cell("int_x")
    frac_x = comp.get_cell("frac_x")
    with comp.group("split_bits") as split_bits:
        and0.left = input.out
        and0.right = 2**width - 2**frac_width
        rsh.left = and0.out
        rsh.right = frac_width
        and1.left = input.out
        and1.right = (2**frac_width) - 1
        int_x.write_en = 1
        frac_x.write_en = 1
        int_x.in_ = rsh.out
        frac_x.in_ = and1.out
        split_bits.done = (int_x.done & frac_x.done) @ 1

    if is_signed:
        mult_pipe = comp.get_cell("mult_pipe1")
        with comp.group("negate") as negate:
            mult_pipe.left = input.out
            mult_pipe.right = comp.get_cell("negative_one").out
            mult_pipe.go = ~mult_pipe.done @ 1
            input.write_en = mult_pipe.done
            input.in_ = mult_pipe.out
            negate.done = input.done

        lt = comp.get_cell("lt")
        with comp.comb_group(name="is_negative"):
            lt.left = comp.this().x
            lt.right = 0

        # Take the reciprocal, since the initial value was -x.
        div_pipe = comp.get_cell("div_pipe")
        input = comp.get_cell("m")
        with comp.group(name="reciprocal") as reciprocal:
            div_pipe.left = 1
            div_pipe.right = input.out
            div_pipe.go = ~div_pipe.done @ 1
            input.write_en = div_pipe.done
            input.in_ = div_pipe.out_quotient
            reciprocal.done = input.done

    for j in range(2, degree + 1):
        consume_pow(comp, j)

    for k in range(2, degree + 1):
        multiply_by_reciprocal_factorial(comp, k)

    divide_and_conquer_sums(comp, degree)
    final_multiply(comp, CompVar("m"))

    # Connect final value to the `out` signal of the component.
    output_register = comp.get_cell("m")
    with comp.continuous:
        comp.this().out = output_register.out


def generate_control(comp: ComponentBuilder, degree: int, is_signed: bool):
    pow_invokes = {
        invoke(
            comp.get_cell("pow1"),
            in_base=comp.get_cell("e").out,
            in_integer_exp=comp.get_cell("int_x").out,
        )
    }.union(
        (
            invoke(
                comp.get_cell(f"pow{i}"),
                in_base=comp.get_cell("frac_x").out,
                in_integer_exp=comp.get_cell(f"c{i}").out,
            )
            for i in range(2, degree + 1)
        )
    )

    consume_pow = {comp.get_group(f"consume_pow{i}") for i in range(2, degree + 1)}
    mult_by_reciprocal = {
        comp.get_group(f"mult_by_reciprocal_factorial{i}") for i in range(2, degree + 1)
    }

    divide_and_conquer = []
    Enable_count = degree >> 1
    for r in range(1, int(log2(degree) + 1)):
        divide_and_conquer.append(
            {comp.get_group(f"sum_round{r}_{i}") for i in range(1, Enable_count + 1)}
        )
        Enable_count >>= 1

    lt = comp.get_cell("lt")
    init = comp.get_group("init")
    split_bits = comp.get_group("split_bits")

    comp.control += [
        init,
        if_(lt.out, comp.get_group("is_negative"), comp.get_group("negate"))
        if is_signed
        else [],
        split_bits,
        pow_invokes,
        consume_pow,
        mult_by_reciprocal,
        divide_and_conquer,
        comp.get_group("add_degree_zero"),
        comp.get_group("final_multiply"),
        if_(lt.out, comp.get_group("is_negative"), comp.get_group("reciprocal"))
        if is_signed
        else [],
    ]


def generate_exp_taylor_series_approximation(
    builder: Builder, degree: int, width: int, int_width: int, is_signed: bool
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

    comp = builder.component("exp")
    comp.input("x", width)
    comp.out("out", width)
    generate_cells(comp, degree, width, int_width, is_signed)
    generate_groups(comp, degree, width, int_width, is_signed)
    generate_control(comp, degree, is_signed)

    return [
        comp.component,
        generate_fp_pow_component(builder, width, int_width, is_signed),
    ]


def gen_reciprocal(
    comp: ComponentBuilder,
    name: str,
    base_cell: CellBuilder,
    div_pipe: CellBuilder,
    const_one: CellBuilder,
):
    """
    Generates a group that takes in a base cell and sets its new value to its reciprocal
    """
    with comp.group(name) as group:
        div_pipe.left = const_one.out
        div_pipe.right = base_cell.out
        div_pipe.go = ~div_pipe.done @ 1
        base_cell.write_en = div_pipe.done
        base_cell.in_ = div_pipe.out_quotient
        group.done = base_cell.done


def gen_reverse_sign(
    comp: ComponentBuilder,
    name: str,
    base_cell: CellBuilder,
    mult_pipe: CellBuilder,
    const_neg_one: CellBuilder,
):
    """
    Generates a group that takes in a base cell and multiplies it by negative one
    """
    with comp.group(name) as group:
        mult_pipe.left = base_cell.out
        mult_pipe.right = const_neg_one.out
        mult_pipe.go = ~mult_pipe.done @ 1
        base_cell.write_en = mult_pipe.done
        base_cell.in_ = mult_pipe.out
        group.done = base_cell.done


def gen_comb_lt(
    comp: ComponentBuilder,
    name: str,
    lhs: ExprBuilder,
    lt: CellBuilder,
    const_cell: CellBuilder,
):
    """
    Generates lhs < const_cell
    """
    with comp.comb_group(name):
        lt.left = lhs
        lt.right = const_cell.out


# This appears to be unused. Brilliant.
# TODO (griffin): Double check that this is unused and, if so, remove it.
def gen_constant_cell(
    comp: ComponentBuilder,
    name: str,
    value: str,
    width: int,
    int_width: int,
    is_signed: bool,
) -> CellBuilder:
    return comp.const(
        name,
        width,
        numeric_types.FixedPoint(
            value, width, int_width, is_signed=is_signed
        ).unsigned_integer(),
    )


def generate_fp_pow_full(
    degree: int, width: int, int_width: int, is_signed: bool
) -> List[Component]:
    """
    Generates a component that can calculate b^x, for any fixed point b and x.
    Here is the idea behind how the component works:
    b^x = e^ln(b^x) = e ^ (x*ln(b)).
    Therefore, we can use our ln component to calculate ln(b) and then multiply
    ln(b) by x. Then we raise that result to the e (using the taylor series approximation)
    and get our result.
    """
    lt = Cell(CompVar("lt"), Stdlib.op("lt", width, is_signed))
    div = Cell(
        CompVar("div_pipe"),
        Stdlib.fixed_point_op(
            "div_pipe", width, int_width, width - int_width, is_signed
        ),
    )
    const_one = Cell(
        CompVar("one"),
        Stdlib.constant(
            width,
            numeric_types.FixedPoint(
                "1.0", width, int_width, is_signed=is_signed
            ).unsigned_integer(),
        ),
    )
    const_zero = Cell(
        CompVar("zero"),
        Stdlib.constant(
            width,
            numeric_types.FixedPoint(
                "0.0", width, int_width, is_signed=is_signed
            ).unsigned_integer(),
        ),
    )
    mult = Cell(
        CompVar("mult"),
        Stdlib.fixed_point_op(
            "mult_pipe", width, int_width, width - int_width, is_signed
        ),
    )
    new_base_reg = Cell(CompVar("new_base"), Stdlib.register(width))
    stored_base_reg = Cell(CompVar("stored_base"), Stdlib.register(width))
    res = Cell(CompVar("res"), Stdlib.register(width))
    base_reciprocal = If(
        port=CompPort(lt.id, "out"),
        cond=CompVar("base_lt_one"),
        true_branch=Enable("set_base_reciprocal"),
    )
    base_rev = If(
        port=CompPort(lt.id, "out"),
        cond=CompVar("base_lt_zero"),
        true_branch=Enable("rev_base_sign"),
    )
    res_rev = If(
        port=CompPort(lt.id, "out"),
        cond=CompVar("base_lt_zero"),
        true_branch=Enable("rev_res_sign"),
    )
    res_reciprocal = If(
        port=CompPort(lt.id, "out"),
        cond=CompVar("base_lt_one"),
        true_branch=Enable("set_res_reciprocal"),
    )

    pre_process = (
        SeqComp([base_rev, Enable("store_old_reg_val"), base_reciprocal])
        if is_signed
        else SeqComp([Enable("store_old_reg_val"), base_reciprocal])
    )
    post_process = (
        SeqComp([res_rev, res_reciprocal]) if is_signed else SeqComp([res_reciprocal])
    )

    if is_signed:
        const_neg_one = Cell(
            CompVar("neg_one"),
            Stdlib.constant(
                width,
                numeric_types.FixedPoint(
                    "-1.0", width, int_width, is_signed=is_signed
                ).unsigned_integer(),
            ),
        )
        rev_structs = [
            gen_reverse_sign("rev_base_sign", new_base_reg, mult, const_neg_one),
            gen_reverse_sign("rev_res_sign", res, mult, const_neg_one),
            gen_comb_lt("base_lt_zero", ThisPort(CompVar("base")), lt, const_zero),
            const_neg_one,
        ]

    return (
        generate_exp_taylor_series_approximation(degree, width, int_width, is_signed)
        + generate_ln(width, int_width, is_signed)
        + [
            Component(
                "fp_pow_full",
                inputs=[
                    PortDef(CompVar("base"), width),
                    PortDef(CompVar("exp_value"), width),
                ],
                outputs=[PortDef(CompVar("out"), width)],
                structs=(rev_structs if is_signed else [])
                + [
                    const_one,
                    mult,
                    new_base_reg,
                    stored_base_reg,
                    res,
                    lt,
                    div,
                    const_zero,
                    Cell(CompVar("new_exp_val"), Stdlib.register(width)),
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
                                ThisPort(CompVar("exp_value")),
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
                            ),
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
                                ThisPort(CompVar("base")),
                            ),
                            Connect(
                                HolePort(CompVar("write_to_base_reg"), "done"),
                                CompPort(new_base_reg.id, "done"),
                            ),
                        ],
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
                                CompPort(new_base_reg.id, "out"),
                            ),
                            Connect(
                                HolePort(CompVar("store_old_reg_val"), "done"),
                                CompPort(stored_base_reg.id, "done"),
                            ),
                        ],
                    ),
                    Group(
                        id=CompVar("write_e_to_res"),
                        connections=[
                            Connect(
                                CompPort(res.id, "write_en"),
                                ConstantPort(1, 1),
                            ),
                            Connect(
                                CompPort(res.id, "in"), CompPort(CompVar("e"), "out")
                            ),
                            Connect(
                                HolePort(CompVar("write_e_to_res"), "done"),
                                CompPort(res.id, "done"),
                            ),
                        ],
                    ),
                    gen_reciprocal("set_base_reciprocal", new_base_reg, div, const_one),
                    gen_reciprocal("set_res_reciprocal", res, div, const_one),
                    gen_comb_lt(
                        "base_lt_one",
                        CompPort(stored_base_reg.id, "out"),
                        lt,
                        const_one,
                    ),
                ],
                controls=SeqComp(
                    [
                        Enable("write_to_base_reg"),
                        pre_process,
                        Invoke(
                            id=CompVar("l"),
                            in_connects=[("x", CompPort(new_base_reg.id, "out"))],
                            out_connects=[],
                        ),
                        Enable("set_new_exp"),
                        Invoke(
                            id=CompVar("e"),
                            in_connects=[
                                ("x", CompPort(CompVar("new_exp_val"), "out"))
                            ],
                            out_connects=[],
                        ),
                        Enable("write_e_to_res"),
                        post_process,
                    ]
                ),
            )
        ]
    )


def build_base_not_e(degree, width, int_width, is_signed) -> Program:
    """
    Builds a program that uses reads from an external memory file to test
    the fp_pow_full component (`fp_pow_full works` for any base, but since
    we already have an `exp` component that works for base `e`, it is better
    to just use that if we want to calculate the base being e).
    """
    program = Program(
        imports=[
            Import("primitives/core.futil"),
            Import("primitives/binary_operators.futil"),
        ],
        components=generate_fp_pow_full(degree, width, int_width, is_signed),
    )
    # main component for testing purposes
    program_main = Component(
        "main",
        inputs=[],
        outputs=[],
        structs=[
            Cell(CompVar("base_reg"), Stdlib.register(width)),
            Cell(CompVar("exp_reg"), Stdlib.register(width)),
            Cell(CompVar("x"), Stdlib.mem_d1(width, 1, 1), is_external=True),
            Cell(CompVar("b"), Stdlib.mem_d1(width, 1, 1), is_external=True),
            Cell(
                CompVar("ret"),
                Stdlib.mem_d1(width, 1, 1),
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
                    in_connects=[
                        ("base", CompPort(CompVar("base_reg"), "out")),
                        ("exp_value", CompPort(CompVar("exp_reg"), "out")),
                    ],
                    out_connects=[],
                ),
                Enable("write_to_memory"),
            ]
        ),
    )
    program.components.append(program_main)
    return program


def build_base_is_e(degree, width, int_width, is_signed) -> Program:
    """
    Builds a program that uses reads from an external memory file to test
    the exp component. Exp can calculate any power as long as the base is e.
    """
    program = Program(
        imports=[
            Import("primitives/core.futil"),
            Import("primitives/binary_operators.futil"),
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
                Cell(CompVar("t"), Stdlib.register(width)),
                Cell(CompVar("x"), Stdlib.mem_d1(width, 1, 1), is_external=True),
                Cell(
                    CompVar("ret"),
                    Stdlib.mem_d1(width, 1, 1),
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
    return program


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
    required_fields = [
        args.degree,
        args.width,
        args.int_width,
        args.is_signed,
        args.base_is_e,
    ]
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

    # build 2 separate programs:
    # if base_is_e is true, then we only need to generate the exp component,
    # which can caclulate e^x for any x.
    # if base_is_e is false, then we need to generate additional hardware; namely,
    # the ln component (in addition to the exp component). Having both ln
    # and e^x available to use allows us to calculate b^x for any b and any x.
    if base_is_e:
        build_base_is_e(degree, width, int_width, is_signed).emit()
    else:
        build_base_not_e(degree, width, int_width, is_signed).emit()
