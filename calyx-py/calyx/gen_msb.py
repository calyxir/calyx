from typing import List
from calyx.py_ast import (
    Component,
)
from calyx.builder import (
    Builder,
    CellAndGroup,
    const,
    HI,
    while_with,
)


def gen_msb_calc(width: int, int_width: int) -> List[Component]:
    """
    Generates Calyx component to produce the following:
    For a given fixed point number x>=1, we want to select the largest n such
    that 2^n <= x. Note that this is essentially finding the index of the most
    significant bit in x. count_ans is the value for `n`, and `value_ans` is the
    value for `2^n`.
    The component uses a while loop, a counter register, and shifts the input
    1 bit to the right at each iteration until it equals 0.
    Important note: this component doesn't work when the input is 0.
    """
    builder = Builder()
    comp = builder.component("msb_calc")
    in_ = comp.input("in", width)
    comp.output("count", width)
    comp.output("value", width)

    counter = comp.reg(width, "counter")
    cur_val = comp.reg(width, "cur_val")
    count_ans = comp.reg(width, "count_ans")
    val_ans = comp.reg(width, "val_ans")
    val_build = comp.reg(width, "val_build")
    rsh = comp.rsh(width)
    neq = comp.neq(width)

    with comp.group("wr_cur_val") as wr_cur_val:
        rsh.left = in_
        rsh.right = const(width, int_width)
        cur_val.in_ = rsh.out
        cur_val.write_en = HI
        wr_cur_val.done = cur_val.done

    with comp.group("wr_val_build") as wr_val_build:
        val_build.in_ = const(32, 1)
        val_build.write_en = HI
        wr_val_build.done = val_build.done

    with comp.comb_group("cur_val_cond") as cur_val_cond:
        neq.left = const(width, 0)
        neq.right = cur_val.out

    with comp.comb_group("count_cond") as count_cond:
        neq.left = const(width, 0)
        neq.right = counter.out

    incr_count = comp.incr(counter)
    decr_count = comp.decr(counter)

    with comp.group("shift_cur_val") as shift_cur_val:
        rsh.left = cur_val.out
        rsh.right = const(width, 1)
        cur_val.in_ = rsh.out
        cur_val.write_en = HI
        shift_cur_val.done = cur_val.done

    shift_val_build = comp.lsh_use(val_build.out, val_build)
    wr_count = comp.lsh_use(counter.out, count_ans, width - int_width)
    wr_val = comp.lsh_use(val_build.out, val_ans, width - int_width)

    with comp.continuous:
        comp.this().count = count_ans.out
        comp.this().value = val_ans.out

    comp.control += [
        wr_cur_val,
        while_with(
            CellAndGroup(neq, cur_val_cond),
            [incr_count, shift_cur_val],
        ),
        decr_count,
        wr_count,
        wr_val_build,
        while_with(
            CellAndGroup(neq, count_cond),
            [decr_count, shift_val_build],
        ),
        wr_val,
    ]

    return [comp.component]
