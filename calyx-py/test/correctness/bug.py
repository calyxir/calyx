import calyx.builder as cb

# BUG: reading from ref seq_mem (passed through invoke)
def component(prog):
    comp = prog.component("comp")
    
    A = comp.seq_mem_d1("A", 32, 1, 1, is_ref=True)
    B = comp.seq_mem_d1("B", 32, 1, 1, is_ref=True)

    a = comp.reg(32) 
    zero = comp.const("zero", 1, 0) 

    read_A = comp.mem_load_d1(A, zero.out, a, "read") 
    write_B = comp.mem_store_d1(B, zero.out, a.out, "write") 

    comp.control += [read_A, write_B]

    return comp

def insert_main(prog):
    main = prog.component("main")
    
    A = main.seq_mem_d1("A", 32, 1, 1, is_external=True)
    B = main.seq_mem_d1("B", 32, 1, 1, is_external=True)

    comp = component(prog)
    comp = main.cell("comp", comp)

    main.control += [cb.invoke(comp, ref_A=A, ref_B=B)]

if __name__ == "__main__":
    prog = cb.Builder()
    insert_main(prog)
    prog.program.emit()

