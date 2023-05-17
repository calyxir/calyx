# pylint: disable=import-error
from calyx.builder import Builder


def build():
    """Just two memories."""
    prog = Builder()
    main = prog.component("main")
    main.input("in", 32)
    main.output("out", 32)

    _ = main.mem_d1("mem_1", 32, 4, 32, is_ref=True)
    _ = main.mem_d1("mem_2", 32, 4, 32, is_ref=True)

    return prog.program


if __name__ == "__main__":
    build().emit()
