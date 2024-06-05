import calyx.builder as cb

def insert_matmul_component(prog):
    matmul = prog.component("main")

    A = matmul.comb_mem_d2("A", 32, 4, 4, 3, 3, is_external=True)
    B = matmul.seq_mem_d2("B", 32, 4, 4, 3, 3, is_external=True)
    C = matmul.seq_mem_d2("C", 32, 4, 4, 3, 3, is_external=True)

    mult = matmul.mult_pipe(32)
    add = matmul.add(32)

    acc = matmul.reg(32)

    # matrix entries
    a = matmul.reg(32)
    b = matmul.reg(32)

    # iterators
    i = matmul.reg(3)
    j = matmul.reg(3)
    k = matmul.reg(3)

    zero_acc = matmul.reg_store(acc, 0)
    zero_i = matmul.reg_store(i, 0)
    zero_j = matmul.reg_store(j, 0)
    zero_k = matmul.reg_store(k, 0)

    cond_i = matmul.lt_use(i.out, 4)
    cond_j = matmul.lt_use(j.out, 4)
    cond_k = matmul.lt_use(k.out, 4)

    read_A = matmul.mem_load_d2(A, i.out, k.out, a, "read_A")
    read_B = matmul.mem_load_d2(B, k.out, j.out, b, "read_B")
    
    write = matmul.mem_store_d2(C, i.out, j.out, acc.out, "write")
    
    with matmul.group("upd") as upd:
        mult.go = 1
        mult.left = a.out
        mult.right = b.out

        add.left = mult.done @ mult.out
        add.right = mult.done @ acc.out
        acc.in_ = mult.done @ add.out

        acc.write_en = mult.done @ cb.HI 
        upd.done = mult.done @ acc.done

    matmul.control += [ 
        zero_i,
        cb.while_with(cond_i, 
            [
                zero_j,
                cb.while_with(cond_j, 
                    [
                        zero_k,
                        zero_acc,
                        cb.while_with(cond_k, [read_A, read_B, upd, matmul.incr(k)]),
                        write,
                        matmul.incr(j)
                    ]),
                matmul.incr(i)
            ])
     ]

if __name__ == "__main__":
    prog = cb.Builder()
    insert_matmul_component(prog)
    prog.program.emit()

