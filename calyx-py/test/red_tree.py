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


def use_tree(comp, group, a, a_i, b, b_i, c, c_i, d, d_i, tree):
    """Orchestrates the use of the component `tree`.
    Adds wiring for {group}, which puts into the tree's four leaves
    the values a[a_i], b[b_i], c[c_i], and d[d_i].
    """
    with comp.group(group) as tree_use:
        a.addr0 = cb.const(32, a_i)
        b.addr0 = cb.const(32, b_i)
        c.addr0 = cb.const(32, c_i)
        d.addr0 = cb.const(32, d_i)
        tree.leaf1 = a.read_data
        tree.leaf2 = b.read_data
        tree.leaf3 = c.read_data
        tree.leaf4 = d.read_data
        tree.go = cb.const(1, 1)
        tree_use.done = tree.done


def add_main(prog):
    """Inserts the component `main` into the program.
    This will be used in concert with multiple copies of the component `tree`.
    It requires:
    - Memories `A`, `B`, `C`, `D`, of length 4 each, to be driven from the data file.
    - A memory `ans` to store the result, also driven from the data file.

    It puts the sum of elements of the four memory cells into `ans`.
    """
    main = prog.component("main")
    A = main.mem_d1("A", 32, 4, 32, is_external=True)
    B = main.mem_d1("B", 32, 4, 32, is_external=True)
    C = main.mem_d1("C", 32, 4, 32, is_external=True)
    D = main.mem_d1("D", 32, 4, 32, is_external=True)
    E = main.mem_d1("ans", 32, 1, 1, is_external=True)

    _ = main.reg("sum_col1", 32)
    _ = main.reg("sum_col2", 32)
    _ = main.reg("sum_col3", 32)
    _ = main.reg("sum_col4", 32)

    # AM, point of failure:
    # I'd like to add the following to the `cells` section:
    # tree0 = tree();
    # I think the following is what you want me to do:
    tree0 = main.cell("tree0", prog.component("tree"))
    # But _it adds a new, blank component called tree_ to the program.
    # I'd like for it to locate the existing component `tree`.
    # Thoughts?
    tree1 = main.cell("tree1", prog.component("tree"))
    tree2 = main.cell("tree2", prog.component("tree"))
    tree3 = main.cell("tree3", prog.component("tree"))
    tree4 = main.cell("tree4", prog.component("tree"))

    use_tree(main, "tree0_col0", A, 0, B, 0, C, 0, D, 0, tree0)
    use_tree(main, "tree1_col1", A, 1, B, 1, C, 1, D, 1, tree1)
    use_tree(main, "tree2_col2", A, 2, B, 2, C, 2, D, 2, tree2)
    use_tree(main, "tree3_col3", A, 3, B, 3, C, 3, D, 3, tree3)


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    add_tree(prog)
    add_main(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
