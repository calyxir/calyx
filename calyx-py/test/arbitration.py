# pylint: disable=import-error
from calyx.builder import Builder


def add_wrap(prog):
    """Inserts the wrap component into the program.
    It just has two memories and nothing else.
    """
    main = prog.component("wrap3")
    main.input("i", 32)
    main.input("j", 32)

    main.output("out", 32)

    _ = main.mem_d1("mem_1", 32, 4, 32, is_ref=True)
    _ = main.mem_d1("mem_2", 32, 4, 32, is_ref=True)


def build():
    """Just two memories."""
    prog = Builder()
    add_wrap(prog)

    return prog.program


if __name__ == "__main__":
    build().emit()
