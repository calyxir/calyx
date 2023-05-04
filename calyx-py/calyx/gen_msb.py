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
    HolePort,
    PortDef,
    SeqComp,
    Enable,
    While,
    Control,
    CombGroup,
)
from calyx.builder import Builder, ComponentBuilder, const, HI, while_


def gen_msb_calc(width: int, int_width: int) -> List[Component]:
    """
    Generates Calyx component to produce the following:
    For a given fixed point number x>=1, we want to select the largest n such
    that 2^n <= x. Note that this is essentially finding the index of the most
    significant bit in x. count_ans is the value for `n`, and `value_ans` is the
    value for `2^n`.
    Essentially, the component uses a while loop, a counter register, and shifts the input
    1 bit to the right at each iteration until it equals 0.
    Important note: this component doesn't work when the input is 0.
    """
    builder = Builder()
    comp = builder.component("msb_calc")
    comp.input("in", width)
    comp.output("count", width)
    comp.output("value", width)

    rsh = comp.cell("rsh", Stdlib.op("rsh", width, signed=False))
    counter = comp.reg("counter", width)
    cur_val = comp.reg("cur_val", width)
    add = comp.cell("add", Stdlib.op("add", width, signed=False))
    sub = comp.cell("sub", Stdlib.op("sub", width, signed=False))
    neq = comp.cell("neq", Stdlib.op("neq", width, signed=False))
    lsh = comp.cell("lsh", Stdlib.op("lsh", width, signed=False))
    count_ans = comp.reg("count_ans", width)
    val_ans = comp.reg("val_ans", width)
    val_build = comp.reg("val_build", width)

    with comp.group("wr_cur_val") as wr_cur_val:
        rsh.left = comp.this().in_
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

    with comp.group("incr_count") as incr_count:
        add.left = counter.out
        add.right = const(width, 1)
        counter.in_ = add.out
        counter.write_en = HI
        incr_count.done = counter.done

    with comp.group("shift_cur_val") as shift_cur_val:
        rsh.left = cur_val.out
        rsh.right = const(width, 1)
        cur_val.in_ = rsh.out
        cur_val.write_en = HI
        shift_cur_val.done = cur_val.done

    with comp.group("shift_val_build") as shift_val_build:
        lsh.left = val_build.out
        lsh.right = const(width, 1)
        val_build.in_ = lsh.out
        val_build.write_en = HI
        shift_val_build.done = val_build.done

    with comp.group("decr_count") as decr_count:
        sub.left = counter.out
        sub.right = const(width, 1)
        counter.in_ = sub.out
        counter.write_en = HI
        decr_count.done = counter.done

    with comp.group("wr_count") as wr_count:
        lsh.left = counter.out
        lsh.right = const(width, width - int_width)
        count_ans.in_ = lsh.out
        count_ans.write_en = HI
        wr_count.done = count_ans.done

    with comp.group("wr_val") as wr_val:
        lsh.left = val_build.out
        lsh.right = const(width, width - int_width)
        val_ans.in_ = lsh.out
        val_ans.write_en = HI
        wr_val.done = val_ans.done

    with comp.continuous:
        comp.this().count = count_ans.out
        comp.this().value = val_ans.out

    comp.control += [
        wr_cur_val,
        while_(
            neq.out,
            cur_val_cond,
            [incr_count, shift_cur_val],
        ),
        decr_count,
        wr_count,
        wr_val_build,
        while_(
            neq.out,
            count_cond,
            [decr_count, shift_val_build],
        ),
        wr_val,
    ]

    return [comp.component]
