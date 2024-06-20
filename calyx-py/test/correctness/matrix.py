import calyx.builder as cb


def insert_matmul_component(prog, n):
    """Inserts the component `matmul` into the program.

    It has:
    - one 2d combinational ref memory, A
    - two 2d sequential ref memories, B and C

    The memories are of a variety of flavors just to demonstrate what is similar
    and different between them; there is not a deep reason we have chosen these.

    By and large we recommend the use of sequential memories, which are more realistic.

    Interpreting A and B as n x n matrices, matmul computes the matrix product
    A*B and writes this into C.
    """

    logn = n.bit_length()

    matmul = prog.component("matmul")

    # The matrices are declared using a similar syntax.
    A = matmul.comb_mem_d2("A", 32, n, n, logn, logn, is_ref=True)
    B = matmul.seq_mem_d2("B", 32, n, n, logn, logn, is_ref=True)
    C = matmul.seq_mem_d2("C", 32, n, n, logn, logn, is_ref=True)

    mult = matmul.mult_pipe(32)
    add = matmul.add(32)
    acc = matmul.reg(32)

    # Iterators: i, j, k ∈ [0, n)
    i = matmul.reg(3)
    j = matmul.reg(3)
    k = matmul.reg(3)

    # A register to store an entry of matrix A. We won't need one for matrix B.
    a = matmul.reg(32)

    zero_acc = matmul.reg_store(acc, 0)  # acc := 0
    zero_j = matmul.reg_store(j, 0)  # j := 0
    zero_k = matmul.reg_store(k, 0)  # k := 0

    cond_i = matmul.lt_use(i.out, n)  # i < n
    cond_j = matmul.lt_use(j.out, n)  # j < n
    cond_k = matmul.lt_use(k.out, n)  # k < n

    # a := A[i][k]
    load_A = matmul.mem_load_d2(A, i.out, k.out, a, "read_A")
    # Latch B[k][j], so that we can later read `B.read_data` and get B[k][j].
    # While `mem_load` works on combinational and sequential memories,
    # `mem_latch` only works on sequential memories.
    latch_B = matmul.mem_latch_d2(B, k.out, j.out, "read_B")

    # C[i][j] := acc
    write_C = matmul.mem_store_d2(C, i.out, j.out, acc.out, "write")
    # We do not demonstrate it here, but storing into a combinational memory
    # works identically. The `mem_store` method is overloaded.

    # acc := acc + (a * b), where b is the value latched at B
    with matmul.group("upd_acc") as upd_acc:
        # Compute a*b
        mult.go = 1
        mult.left = a.out
        mult.right = B.read_data

        # Compute acc + (a*b)
        add.left = mult.done @ acc.out
        add.right = mult.done @ mult.out

        # acc := acc + (a*b)
        acc.in_ = mult.done @ add.out
        acc.write_en = mult.done @ cb.HI

        upd_acc.done = mult.done @ acc.done

    matmul.control += [
        cb.while_with(
            cond_i,
            [
                zero_j,
                cb.while_with(
                    cond_j,
                    [
                        zero_k,
                        zero_acc,
                        cb.while_with(
                            cond_k,
                            [
                                load_A,
                                latch_B,
                                upd_acc,
                                matmul.incr(k),
                            ],
                        ),
                        write_C,
                        matmul.incr(j),
                    ],
                ),
                matmul.incr(i),
            ],
        ),
    ]

    return matmul


def insert_main(prog):
    main = prog.component("main")

    n = 4
    logn = n.bit_length()

    A = main.comb_mem_d2("A", 32, n, n, logn, logn, is_external=True)
    B = main.seq_mem_d2("B", 32, n, n, logn, logn, is_external=True)
    C = main.seq_mem_d2("C", 32, n, n, logn, logn, is_external=True)

    matmul = insert_matmul_component(prog, n)
    matmul = main.cell("matmul", matmul)

    main.control += [cb.invoke(matmul, ref_A=A, ref_B=B, ref_C=C)]


if __name__ == "__main__":
    prog = cb.Builder()
    insert_main(prog)
    prog.program.emit()
