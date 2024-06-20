from calyx.builder import Builder, invoke

#Creates a component the has a case statement.
def add_case(prog):
    # Inputs/Outputs
    my_comp = prog.component("my_comp")
    comp_reg = my_comp.reg(1, "comp_reg")
    in_1 = my_comp.input("in_1", 8)
    out_1 = my_comp.output("out_1", 16)

    with my_comp.group("my_group") as my_group:
        # Some assignments
        my_comp.out_1 = 24

    my_invoke = invoke(comp_reg, in_in=1)
    in_1_comps = my_comp.case(in_1, {1: my_group, 2: my_invoke})
    my_comp.control += in_1_comps


def build():
    prog = Builder()
    add_case(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()

