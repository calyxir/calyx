# This is meant to test the `__add__` and `__mul__` functionality of
# the builder's ControlBuilder. In particular we look at `+=` and `*=`
from calyx.builder import (
    Builder,
    par,
)


def add_par_thing(prog):
    my_comp = prog.component("my_comp")

    my_group = my_comp.group("my_group")
    my_group2 = my_comp.group("my_group2")
    my_group3 = my_comp.group("my_group3")

    my_par = par(my_group2, my_group3)

    my_comp.control *= my_group
    # Make sure that an ast.ParComp and CompBuilder par get flattened.
    my_comp.control *= my_par
    my_comp.control *= par(my_group2, my_group3)
    # Turn into seq of par group then [my_group, my_group2]
    my_comp.control += [my_group, my_group2]
    my_comp.control += my_group
    my_comp.control += [my_group3]
    # Check going from seq to par block
    my_comp.control *= [my_group2, my_group3]


def build():
    prog = Builder()
    add_par_thing(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
