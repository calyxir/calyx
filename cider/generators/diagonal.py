from calyx.builder import (
    HI,
    Builder,
    CellBuilder,
    ComponentBuilder,
    par,
    while_with,
)


GRID_COLS = 256
BSIZE = 16
# Arrays are BSIZE x GRID_COLS


mem_args = {
    "bitwidth": 32,
    "len0": BSIZE,
    "len1": GRID_COLS,
    "idx_size0": 32,
    "idx_size1": 32,
}


buf_1_mems = []
buf_2_mems = []


def inner_loop(
    main: ComponentBuilder,
    m: int,
    # n_reg: CellBuilder,
    tmp_reg: CellBuilder,
    loop_prefix: str,
    buf1_access,
    buf2_access,
):
    first_half = []
    seq_compute = []
    for k in range(0, m):
        buffer_up_1 = main.seq_mem_d2(f"buffer_up_1{loop_prefix}_{m}_{k}", **mem_args)
        buffer_up_2 = main.seq_mem_d2(f"buffer_up_2{loop_prefix}_{m}_{k}", **mem_args)
        buf_1_mems.append(buffer_up_1.name)
        buf_2_mems.append(buffer_up_2.name)

        latch_1 = main.mem_latch_d2(
            buffer_up_1,
            k,
            buf1_access,
            f"read_buf1{loop_prefix}_{m}_{k}",
        )
        latch_2 = main.mem_latch_d2(
            buffer_up_2,
            buf2_access,
            k,
            f"read_buf2{loop_prefix}_{m}_{k}",
        )

        (mul, mul_reg) = main.mult_store_in_reg(
            buffer_up_2.read_data, buffer_up_1.read_data, width=32
        )
        first_half.append([par(latch_1, latch_2), mul])
        (compute, _) = main.sub_store_in_reg(tmp_reg.out, mul_reg.out, tmp_reg)
        seq_compute.append(compute)

    return [par(*first_half), *seq_compute]


def middle_loop1(main: ComponentBuilder, m: int):
    buffer_up_1 = main.seq_mem_d2(f"buffer_up_1_l1_{m}", **mem_args)
    buffer_up_2 = main.seq_mem_d2(f"buffer_up_2_l1_{m}", **mem_args)
    buf_1_mems.append(buffer_up_1.name)
    buf_2_mems.append(buffer_up_2.name)

    tmp = main.reg(32, f"tmp_l1_{m}")

    n_reg = main.reg(32, f"n_reg_l1_{m}")
    load_initial = main.reg_store(n_reg, m)

    cond = main.lt_use(n_reg.out, BSIZE)

    load = main.mem_load_d2(buffer_up_1, m, n_reg.out, tmp, f"load_temp_l1_{m}")
    inner = inner_loop(main, m, tmp, "l1", n_reg.out, m)
    write_buf1 = main.mem_store_d2(
        buffer_up_1, m, n_reg.out, tmp.out, f"write_b1_l1_{m}"
    )
    write_buf2 = main.mem_store_d2(
        buffer_up_2, m, n_reg.out, tmp.out, f"write_b2_l1_{m}"
    )
    incr = main.incr(n_reg)

    return [
        load_initial,
        while_with(cond, [load, inner, par(write_buf1, write_buf2), incr]),
    ]


def middle_loop2(main: ComponentBuilder, m: int):
    buffer_up_1 = main.seq_mem_d2(f"buffer_up_1_l2_{m}", **mem_args)
    buffer_up_2 = main.seq_mem_d2(f"buffer_up_2_l2_{m}", **mem_args)
    buf_1_mems.append(buffer_up_1.name)
    buf_2_mems.append(buffer_up_2.name)

    tmp = main.reg(32, f"tmp_l2_{m}")

    n_reg = main.reg(32, f"n_reg_l2_{m}")
    load_initial = main.reg_store(n_reg, m)

    cond = main.lt_use(n_reg.out, BSIZE)

    load = main.mem_load_d2(buffer_up_1, n_reg.out, m, tmp, f"load_temp_l2_{m}")
    inner = inner_loop(main, m, tmp, "l2", m, n_reg.out)

    load_reg = main.reg(32, f"l2_{m}_reg")

    load_read = main.mem_load_d2(buffer_up_1, m, m, load_reg, f"latch_l2_{m}")
    div_reg = main.reg(32, f"div_reg_{m}")
    divider = main.div_pipe(32, f"div_pipe_{m}")

    with main.group(f"do_div_{m}") as divide_group:
        divider.go = HI
        divider.left = tmp.out
        divider.right = load_reg.out
        div_reg.in_ = divider.done @ divider.out_quotient
        div_reg.write_en = divider.done
        divide_group.done = div_reg.done

    write_buf1 = main.mem_store_d2(
        buffer_up_1, n_reg.out, m, div_reg.out, f"write_b1_l2_{m}"
    )
    write_buf2 = main.mem_store_d2(
        buffer_up_2, n_reg.out, m, div_reg.out, f"write_b2_l2_{m}"
    )
    incr = main.incr(n_reg)

    return [
        load_initial,
        while_with(
            cond,
            [load, inner, load_read, divide_group, par(write_buf1, write_buf2), incr],
        ),
    ]


def outer_loop_unrolled(main: ComponentBuilder):
    return par(
        *([middle_loop1(main, m), middle_loop2(main, m)] for m in range(0, BSIZE))
    )


def outer_loop_seq(main: ComponentBuilder):
    return [[middle_loop1(main, m), middle_loop2(main, m)] for m in range(0, BSIZE)]


def main():
    b = Builder()
    b.import_("primitives/memories/seq.futil")
    b.import_("primitives/binary_operators.futil")
    main = b.component("main")

    buffer_up_1 = main.seq_mem_d2("buffer_up_1", is_external=True, **mem_args)
    buffer_up_2 = main.seq_mem_d2("buffer_up_2", is_external=True, **mem_args)
    buf_1_mems.append(buffer_up_1.name)
    buf_2_mems.append(buffer_up_2.name)

    main.control += outer_loop_unrolled(main)

    print(f"// --entangle '{','.join(buf_1_mems)}' --entangle '{','.join(buf_2_mems)}'")

    b.program.emit()


if __name__ == "__main__":
    main()
