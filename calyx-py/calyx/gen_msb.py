from typing import List
from calyx.py_ast import Component
from calyx.builder import Builder, while_with


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

    counter = comp.reg(width)
    cur_val = comp.reg(width)
    count_ans = comp.reg(width)
    val_ans = comp.reg(width)
    val_build = comp.reg(width)

    wr_cur_val = comp.rsh_use(in_, cur_val, int_width)
    wr_val_build = comp.reg_store(val_build, 1)
    cur_val_cond = comp.neq_use(0, cur_val.out)
    count_cond = comp.neq_use(0, counter.out)
    incr_count = comp.incr(counter)
    decr_count = comp.decr(counter)

    shift_cur_val = comp.rsh_use(cur_val.out, cur_val)
    shift_val_build = comp.lsh_use(val_build.out, val_build)
    wr_count = comp.lsh_use(counter.out, count_ans, width - int_width)
    wr_val = comp.lsh_use(val_build.out, val_ans, width - int_width)

    with comp.continuous:
        comp.this().count = count_ans.out
        comp.this().value = val_ans.out

    comp.control += [
        wr_cur_val,
        while_with(
            cur_val_cond,
            [incr_count, shift_cur_val],
        ),
        decr_count,
        wr_count,
        wr_val_build,
        while_with(
            count_cond,
            [decr_count, shift_val_build],
        ),
        wr_val,
    ]

    return [comp.component]
