# pylint: disable=import-error
from calyx.py_ast import Stdlib, CompPort, CompVar, ParComp, SeqComp, Enable
import calyx.builder as cb


def add_adder(
    comp,
    group,
    port_l,
    port_r,
    cell,
    ans,
):
    """Adds wiring for {group}, which puts {port_l} and {port_r}
    into the adder {cell}. It then puts the output of {cell} into
    the memory cell {ans}.
    """
    # AM, point of failure:
    # If passed a {cell} name that's already defined,
    # it creates a duplicate cell instead of locating and reusing the existing one.
    adder = comp.cell(cell, Stdlib.op("add", 32, signed=False))
    with comp.group(group) as adder_group:
        adder.left = CompPort(CompVar(port_l), "out")
        adder.right = CompPort(CompVar(port_r), "out")
        # AM, point of failure:
        # This is wrong. It renders "{port_l}.out" but I want "{port_l}".
        ans.write_en = 1
        ans.in_ = adder.out
        adder_group.done = ans.done


def add_tree(prog):
    """Inserts the component `tree` into the program.
    It has:
    - four inputs, `leaf1`, `leaf2`, `leaf3`, and `leaf4`
    - one output, `sum`

    When done, it puts the sum of the four leaves into `sum`.
    """

    tree = prog.component("tree")
    for i in range(1, 5):
        tree.input(f"leaf{i}", 32)
    tree.output("sum", 32)

    root = tree.reg("root", 32)
    left = tree.reg("left_node", 32)
    right = tree.reg("right_node", 32)

    add_adder(tree, "add_l1_l2", "leaf1", "leaf2", "add1", left)
    add_adder(tree, "add_l3_l4", "leaf3", "leaf4", "add2", right)
    add_adder(tree, "add_left_right_nodes", "left_node", "right_node", "add3", root)

    tree.control = SeqComp(
        [
            ParComp([Enable("add_l1_l2"), Enable("add_l3_l4")]),
            Enable("add_left_right_nodes"),
        ]
    )


def add_main(prog):
    """Inserts the component `main` into the program.
    This will be used in concert with multiple copies of the component `tree`.
    It requires:
    - Memories `A`, `B`, `C`, `D`, of length 4 each, to be driven from the data file.
    - A memory `ans` to store the result, also driven from the data file.

    It puts the sum of elements of the four memory cells into `ans`.
    """
    main = prog.component("main")
    _ = main.mem_d1("A", 32, 4, 32, is_external=True)
    _ = main.mem_d1("B", 32, 4, 32, is_external=True)
    _ = main.mem_d1("C", 32, 4, 32, is_external=True)
    _ = main.mem_d1("D", 32, 4, 32, is_external=True)
    _ = main.mem_d1("ans", 32, 1, 1, is_external=True)

    _ = main.reg("sum_col1", 32)
    _ = main.reg("sum_col2", 32)
    _ = main.reg("sum_col3", 32)
    _ = main.reg("sum_col4", 32)

    # AM, point of failure (but a stupid hack has worked):
    # I'd like to add the following to the `cells` section:
    # tree0 = tree();
    # The following is clearly a hilarious hack:
    _ = main.cell("tree0", CompVar("tree()"))
    _ = main.cell("tree1", CompVar("tree()"))
    _ = main.cell("tree2", CompVar("tree()"))
    _ = main.cell("tree3", CompVar("tree()"))
    _ = main.cell("tree4", CompVar("tree()"))
    # (I've just put parentheses in the name of the component.)

    # I think the following is what you actually want me to do:
    # _ = main.cell("tree0", prog.component("tree"))
    # But _it adds a new, blank component called tree_ to the program.
    # I'd like for it to locate the existing component `tree`.
    # Thoughts?


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    add_tree(prog)
    add_main(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
