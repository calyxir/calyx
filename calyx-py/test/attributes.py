from calyx.builder import Builder
def add_attr_check(prog):
    my_comp = prog.component("m_bresp_channel")
    my_comp.attribute("attr1", 7)
    my_comp.attribute("attr2", 1)


def build():
    prog = Builder()
    add_attr_check(prog)
    return prog.program

if __name__ == "__main__":
    build().emit()
