import calyx.builder as cb


def insert_dummy_component(prog):
    comp = prog.component("dummy")
    comp.input("go", 1)
    comp.output("done", 1)
    with comp.continuous:
        comp.this().flamingo = cb.HI


if __name__ == "__main__":
    prog = cb.Builder()
    insert_dummy_component(prog)
    prog.program.emit()
