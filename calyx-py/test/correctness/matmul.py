import calyx.builder as cb
    
def matmul_component(prog, dim):    
    comp = prog.component("main")

    #Constant stores square matrix's dimensions, but can be modified
    mem_dim = (32, dim, dim, 32, 32)

    #Memory components
    matrices = comp.seq_mem_d2("m1", *mem_dim), comp.seq_mem_d2("m2", *mem_dim)
    result = comp.seq_mem_d2("result", *mem_dim)

    #For tracking matrix indices
    row_index = comp.reg(32, "row_index")
    col_index = comp.reg(32, "col_index")
    loop_index = comp.reg(32, "loop_index")

    #For tracking the matrix entries
    trackers = comp.reg(32), comp.reg(32)
    
    #Tracks product and total cell entry
    product_tracker = comp.reg(32, "product_tracker")
    cell_total = comp.reg(32, "cell_total")

    #Loop guards
    lt1 = comp.lt_use(row_index.out, dim)
    lt2 = comp.lt_use(col_index.out, dim)
    lt3 = comp.lt_use(loop_index.out, dim)
    
    #Groups for resetting varies trackers
    reset_col = comp.reg_store(col_index, 0)
    reset_cell_total = comp.reg_store(cell_total, 0)
    reset_loop_idx = comp.reg_store(loop_index, 0)

    #Groups for incrementing row, column and loop indices
    incr_col = comp.incr(col_index)
    incr_row = comp.incr(row_index)
    incr_loop = comp.incr(loop_index)
    
    #From memory, access the latest pair of entries to multiply for matrices 1 and 2
    load_m1 = comp.mem_load_d2(matrices[0], row_index.out, loop_index.out, trackers[0], "load_m1")
    load_m2 = comp.mem_load_d2(matrices[1], loop_index.out, col_index.out, trackers[1], "load_m2")

    #Write result to memory with specified row/column indices
    write = comp.mem_store_d2(result, row_index.out, col_index.out, cell_total.out, "write")

    #Perform multiplication of two elements
    cellwise_product, product_tracker = comp.mult_store_in_reg(trackers[0].out, trackers[1].out, ans_reg=product_tracker, cellname="mult")

    #Add latest cell product into accumulated total
    compute_entry, cell_total = comp.add_store_in_reg(product_tracker.out, cell_total.out, ans_reg=cell_total, cellname="adder")

    #Control sequence
    comp.control += cb.while_with(
        lt1,cb.seq(
            cb.while_with(
                lt2, cb.seq(
                     cb.while_with(
                        lt3, cb.seq(
                            cb.par(load_m1, load_m2),
                            cellwise_product, compute_entry, incr_loop
                        )
                    ), write, cb.par(incr_col, reset_cell_total, reset_loop_idx)
                )
            ), cb.par(incr_row, reset_col)
        )
    )

if __name__ == "__main__":
    prog = cb.Builder()
    matmul_component(prog, 2)
    prog.program.emit()