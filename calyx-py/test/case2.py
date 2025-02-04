import calyx.builder as cb

def insert_identity_component(prog):
    identity = prog.component("identity")
    r = identity.reg(32, "reg")
    in_1 = identity.input("in_1", 32)
    identity.output("out", 32)

    with identity.group("save") as save:
        r.in_ = in_1
        r.write_en = cb.HI
        save.done = r.done
    
    with identity.continuous:
        identity.this().out = r.out

    identity.control += save

    return identity

def make_program(prog):
    main = prog.component("main")
    mem = main.comb_mem_d1("mem", 32, 10, 32)
    reg = main.reg(32, "reg")
    ans = main.reg(32, "ans")
    id_component = insert_identity_component(prog)
    # make 5 versions of ident
    num_ident = 5
    ids = []
    for i in range(1, 1 + num_ident):
        ids.append(main.cell(f"id_{i}", id_component))
    
    # group to read from the memory
    with main.group("read") as read:
        mem.addr0 = cb.LO
        reg.in_ = mem.read_data
        reg.write_en = cb.HI
        read.done = reg.done


    main.control += main.case(reg.out, {n : cb.invoke(ids[n], in_in_1=reg.out, out_out=ans.in_) for n in range(num_ident)})


def build():
    prog = cb.Builder()
    make_program(prog)
    return prog.program

if __name__ == "__main__":
    build().emit()