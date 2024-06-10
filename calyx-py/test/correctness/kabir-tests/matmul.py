import calyx.builder as cb
    
def matmul_component(prog):

    def set_loop_guard(cell, tracker, size):
        cell.left = tracker.out
        cell.right = size
        return cell
    
    def incr_cell(cell, adder):
        adder.left = 1
        adder.right = cell.out
        cell.in_ = adder.out
        cell.write_en = 1
        return cell
    
    comp = prog.component("main")

    #Constant stores square matrix's dimensions, but can be modified
    # 32 = comp.const(32, 3)
    mem_dim = (32, 3, 3, 32, 32)

    #Memory components
    m1 = comp.comb_mem_d2("m1", *mem_dim)
    m2 = comp.comb_mem_d2("m2", *mem_dim)
    result = comp.comb_mem_d2("result", *mem_dim)

    #ALU Components
    adder = comp.add(32)
    multiplier = comp.mult_pipe(32)
    lt1, lt2, lt3 = comp.lt(32, "lt1"), comp.lt(32, "lt2"), comp.lt(32, "lt3")

    #For tracking matrix indices
    row_index, col_index, loop_index = comp.reg(32, "row_index"), comp.reg(32, "col_index"), comp.reg(32, "loop_index")

    #For tracking the matrix entries
    m1_tracker, m2_tracker = comp.reg(32, "m1_tracker"), comp.reg(32, "m2_tracker")
    
    #Tracks product and total cell entry
    product_tracker = comp.reg(32, "product_tracker")
    cell_total = comp.reg(32, "cell_total")


    with comp.comb_group("loop_guard_1") as lg1:
        lt1 = set_loop_guard(lt1, row_index, 3)

    with comp.comb_group("loop_guard_2") as lg2:
        # comp.lt_use(col_index, 3)
        lt2 = set_loop_guard(lt2, col_index, 3)

    with comp.comb_group("loop_guard_3") as lg3:
        lt3 = set_loop_guard(lt3, loop_index, 3)
    
    with comp.group("reset_col") as reset_col:
        col_index.in_ = 0
        col_index.write_en = 1
        reset_col.done = col_index.done

    with comp.group("reset_trackers") as reset_trackers:
        cell_total.in_ = 0
        cell_total.write_en = 1

        loop_index.in_ = cell_total.done @ 0
        loop_index.write_en = cell_total.done @ 1

        reset_trackers.done = loop_index.done

    with comp.group("incr_col") as incr_col:
        col_index = incr_cell(col_index, adder)
        incr_col.done = col_index.done
    
    with comp.group("incr_row") as incr_row:
        row_index = incr_cell(row_index, adder)
        incr_row.done = row_index.done

    with comp.group("incr_loop") as incr_loop:
        loop_index = incr_cell(loop_index, adder)
        incr_loop.done = loop_index.done

    #From memory, access the latest pair of entries to multiply
    with comp.group("access_entries") as access_entries:
        #Position of left element
        m1.addr0 = row_index.out
        m1.addr1 = loop_index.out

        #Position of right element
        m2.addr0 = loop_index.out
        m2.addr1 = col_index.out

        m1_tracker.in_ = m1.read_data
        m1_tracker.write_en = 1

        m2_tracker.in_ = m1_tracker.done @ m2.read_data
        m2_tracker.write_en = m1_tracker.done @ 1

        access_entries.done = m2_tracker.done

        #Perform multiplication of two elements
        with comp.group("compute_cellwise_product") as cellwise_product:
            multiplier.left = m1_tracker.out
            multiplier.right = m2_tracker.out
            multiplier.go = 1;

            product_tracker.in_ = multiplier.done @ multiplier.out
            product_tracker.write_en = multiplier.done @ 1

            cellwise_product.done = product_tracker.done

        #Add latest cell product into accumulated total
        with comp.group("compute_entry") as compute_entry:
            adder.left = product_tracker.out
            adder.right = cell_total.out

            cell_total.in_ = adder.out
            cell_total.write_en = 1

            compute_entry.done = cell_total.done

        #Write result to memory with specified row/column indices
        with comp.group("write") as write:
            result.addr0 = row_index.out
            result.addr1 = col_index.out
            result.write_data = cell_total.out
            result.write_en = 1

            write.done = result.done

    comp.control += cb.while_with(
        cb.CellAndGroup(lt1, lg1),
        cb.seq(
            cb.while_with(
                cb.CellAndGroup(lt2, lg2),
                cb.seq(
                    cb.while_with(
                        cb.CellAndGroup(lt3, lg3),
                        cb.seq(
                            access_entries, cellwise_product, compute_entry, incr_loop
                        )
                    ), write, cb.par(incr_col, reset_trackers)
                )
            ), cb.par(incr_row, reset_col)
        )
    )

if __name__ == "__main__":
    prog = cb.Builder()
    matmul_component(prog)
    prog.program.emit()