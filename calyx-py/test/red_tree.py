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
    # AM, minor:
    # If passed a {cell} name that's already defined,
    # it creates a duplicate cell instead of locating and reusing the existing one.
    # This is minor because it is probably compiled away.
    adder = comp.cell(cell, Stdlib.op("add", 32, signed=False))
    with comp.group(group) as adder_group:
        adder.left = CompPort(CompVar(port_l), "out")
        adder.right = CompPort(CompVar(port_r), "out")
        # AM, point of failure:
        # This is wrong. It renders "{port_l}.out" but I want "{port_l}".
        # The names are misnomers, slash aspirational:
        # I _want_ to pass ports, but am passing names and then accessing ports.
        # In `add_tree_ports_provided` below, I do pass ports.
        # That doesn't seem to be possible here:
        # when invoking this method, I'd like to pass _input_ ports
        # of a component.
        # I don't know how to get a handle on those.
        # Is there a way to look up a port by name?
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
    # AM, point of failure:
    # I need to add the continuous assignment:
    # sum = root.out;
    # But I don't know how to do that.

    tree.control = SeqComp(
        [
            ParComp([Enable("add_l1_l2"), Enable("add_l3_l4")]),
            Enable("add_left_right_nodes"),
        ]
    )


def use_tree_ports_calculated(
    comp, group, a, a_i, b, b_i, c, c_i, d, d_i, tree, ans_reg
):
    """Orchestrates the use of the component `tree`.
    Adds wiring for {group}, which puts into the tree's four leaves
    the values a[a_i], b[b_i], c[c_i], and d[d_i].
    It then runs the tree, and when the tree is done, stores the answer in {ans_reg}.
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
        # AM, point of failure: I need to add the following:
        #   sum_col0.write_en = tree.done ? 1'b1;
        #   sum_col0.in = tree.done ? tree.sum;
        # But I don't know how to do that.
        ans_reg.write_en = 1
        ans_reg.in_ = tree.sum
        tree_use.done = ans_reg.done


def use_tree_ports_provided(comp, group, p1, p2, p3, p4, tree, ans_mem):
    """Orchestrates the use of the component `tree`.
    Adds wiring for {group}, which puts into the tree's four leaves
    the values p1, p2, p3, and p4.
    It then runs the tree, and stores the answer in the std_mem {ans_mem}.
    """
    # i.e., much like the above, but instead of calculating the
    # ports, it takes them as arguments.

    with comp.group(group) as tree_use:
        tree.leaf1 = p1
        tree.leaf2 = p2
        tree.leaf3 = p3
        tree.leaf4 = p4
        tree.go = cb.const(1, 1)
        # AM, point of failure:
        # Just the like the point of failure above,
        # I need the next three assignments to all be guarded by tree.done.
        ans_mem.addr0 = cb.const(1, 0)
        ans_mem.write_data = tree.sum
        ans_mem.write_en = 1
        tree_use.done = ans_mem.done


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
    ans = main.mem_d1("ans", 32, 1, 1, is_external=True)
    sum_col0 = main.reg("sum_col0", 32)
    sum_col1 = main.reg("sum_col1", 32)
    sum_col2 = main.reg("sum_col2", 32)
    sum_col3 = main.reg("sum_col3", 32)

    # AM, point of failure:
    # I'd like to add the following to the `cells` section:
    # tree0 = tree();
    # I think the following is what you want me to do:
    tree = main.cell("tree", prog.component("tree"))
    # But _it adds a new, blank component called tree_ to the program.
    # I'd like for it to locate the existing component `tree`.
    # Thoughts?

    for i, ans_reg in enumerate([sum_col0, sum_col1, sum_col2, sum_col3]):
        use_tree_ports_calculated(
            main, f"add_col{i}", A, i, B, i, C, i, D, i, tree, ans_reg
        )

    use_tree_ports_provided(
        main,
        "add_intermediates",
        sum_col0.out,
        sum_col1.out,
        sum_col2.out,
        sum_col3.out,
        tree,
        ans,
    )

    main.control = SeqComp(
        [
            Enable("tree0_col0"),
            Enable("tree1_col1"),
            Enable("tree2_col2"),
            Enable("tree3_col3"),
            Enable("add_intermediates"),
        ]
    )


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    add_tree(prog)
    add_main(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
