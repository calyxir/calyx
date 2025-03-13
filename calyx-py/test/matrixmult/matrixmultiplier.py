import calyx.builder as cb
import os


def main(prog: cb.Builder):
    comp: cb.ComponentBuilder = prog.component("main")

    in1_mem = comp.comb_mem_d2("in1_mem", 32, 2, 2, 32, 32)
    in2_mem = comp.comb_mem_d2("in2_mem", 32, 2, 2, 32, 32)
    out_mem = comp.comb_mem_d2("out_mem", 32, 2, 2, 32, 32)

    a_reg = comp.reg(32)
    b_reg = comp.reg(32)
    i_reg = comp.reg(32)
    j_reg = comp.reg(32)
    k_reg = comp.reg(32)
    incr_i = comp.incr(i_reg)
    incr_j = comp.incr(i_reg)
    incr_k = comp.incr(i_reg)
    i_lt_2 = comp.lt_use(i_reg.out, 2)
    j_lt_2 = comp.lt_use(j_reg.out, 2)
    k_lt_2 = comp.lt_use(k_reg.out, 2)
    sum_reg = comp.reg(32)
    mult_reg = comp.reg(32)
    sum, _ = comp.add_store_in_reg(sum_reg.out, mult_reg.out, sum_reg)
    mult, _ = comp.mult_store_in_reg(a_reg.out, b_reg.out, mult_reg)

    with comp.group("loada") as load_a:
        in1_mem.addr0 = i_reg.out
        in1_mem.addr1 = k_reg.out
        a_reg.in_ = in1_mem.read_data
        a_reg.write_en = cb.HI
        load_a.done = a_reg.done

    with comp.group("loadb") as load_b:
        in2_mem.addr0 = k_reg.out
        in2_mem.addr1 = j_reg.out
        b_reg.in_ = in2_mem.read_data
        a_reg.write_en = cb.HI
        load_b.done = b_reg.done

    with comp.group("reseti") as reset_i:
        i_reg.in_ = 0
        i_reg.write_en = cb.HI
        reset_i.done = i_reg.done

    with comp.group("resetj") as reset_j:
        j_reg.in_ = 0
        j_reg.write_en = cb.HI
        reset_j.done = j_reg.done

    with comp.group("resetk") as reset_k:
        k_reg.in_ = 0
        k_reg.write_en = cb.HI
        reset_k.done = k_reg.done

    with comp.group("resetsum") as reset_sum:
        sum_reg.in_ = 0
        sum_reg.write_en = cb.HI
        reset_sum.done = sum_reg.done

    with comp.group("updatemem") as update_mem:
        out_mem.addr0 = i_reg.out
        out_mem.addr1 = j_reg.out
        out_mem.write_data = sum_reg.out
        out_mem.write_en = cb.HI
        update_mem.done = out_mem.done

    comp.control += cb.seq(
        reset_i,
        cb.while_with(
            i_lt_2,
            [
                reset_j,
                cb.while_with(
                    j_lt_2,
                    [
                        cb.par(reset_sum, reset_k),
                        cb.while_with(
                            k_lt_2, [cb.par(load_a, load_b), mult, sum, incr_k]
                        ),
                        update_mem,
                        incr_j,
                    ],
                ),
                incr_i,
            ],
        ),
    )


if __name__ == "__main__":
    prog = cb.Builder(fileinfo_base_path=os.path.dirname(os.path.realpath(__file__)))
    main(prog)
    prog.program.emit()
