# pylint: disable=import-error
from calyx.py_ast import CompInst, ParComp, SeqComp, Enable
import calyx.builder as cb


def add_adder(
    comp: cb.ComponentBuilder,
    adder: cb.CellBuilder,
    group,
    port_l,
    port_r,
    ans,
):
    """To component {comp}, adds wiring for an adder-group called {group}.
    Assumes the adder cell {adder} is in the component, and puts {port_l} and {port_r}
    into the adder.
    Then puts the output of {adder} into the memory register {ans}.
    """
    with comp.group(group) as adder_group:
        adder.left = port_l
        adder.right = port_r
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

    tree: cb.ComponentBuilder = prog.component("tree")
    for i in range(1, 5):
        tree.input(f"leaf{i}", 32)
    # AM, quality of life:
    # `input` has no return value, so I'm forced to immediately call `this()`
    # to get the port I just created.
    [leaf1, leaf2, leaf3, leaf4] = [
        tree.this()[name] for name in ["leaf1", "leaf2", "leaf3", "leaf4"]
    ]

    tree.output("sum", 32)

    root = tree.reg("root", 32)
    left = tree.reg("left_node", 32)
    right = tree.reg("right_node", 32)

    add1 = tree.add("add1", 32)
    add2 = tree.add("add2", 32)

    add_adder(tree, add1, "add_l1_l2", leaf1, leaf2, left)
    add_adder(tree, add2, "add_l3_l4", leaf3, leaf4, right)
    add_adder(tree, add1, "add_left_right_nodes", left.out, right.out, root)

    with tree.continuous:
        tree.this().sum = root.out

    tree.control = SeqComp(
        [
            ParComp([Enable("add_l1_l2"), Enable("add_l3_l4")]),
            Enable("add_left_right_nodes"),
        ]
    )


def use_tree_ports_provided(comp, group, port1, port2, port3, port4, tree, ans_mem):
    """Orchestrates the use of the component `tree`.
    Adds wiring for {group}, which puts into the tree's four leaves
    the values p1, p2, p3, and p4.
    It then runs the tree, and stores the answer in the std_mem {ans_mem}.
    """
    # i.e., much like the above, but instead of calculating the
    # ports, it takes them as arguments.

    with comp.group(group) as tree_use:
        tree.leaf1 = port1
        tree.leaf2 = port2
        tree.leaf3 = port3
        tree.leaf4 = port4
        tree.go = cb.HI
        ans_mem.addr0 = tree.done @ 0
        ans_mem.write_data = tree.done @ tree.sum
        ans_mem.write_en = tree.done @ 1
        tree_use.done = ans_mem.done


def use_tree_ports_calculated(
    comp, group, mem_a, mem_b, mem_c, mem_d, i, tree, ans_reg
):
    """Orchestrates the use of the component `tree`.
    Adds wiring for {group}, which puts into the tree's four leaves
    the values a[i], b[i], c[i], and d[i].
    It then runs the tree, and when the tree is done, stores the answer in {ans_reg}.
    """
    with comp.group(group) as tree_use:
        mem_a.addr0 = cb.const(32, i)
        mem_b.addr0 = cb.const(32, i)
        mem_c.addr0 = cb.const(32, i)
        mem_d.addr0 = cb.const(32, i)
        tree.leaf1 = mem_a.read_data
        tree.leaf2 = mem_b.read_data
        tree.leaf3 = mem_c.read_data
        tree.leaf4 = mem_d.read_data
        tree.go = cb.HI
        ans_reg.write_en = tree.done @ 1
        ans_reg.in_ = tree.done @ tree.sum
        tree_use.done = ans_reg.done


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

    # AM, quality of life: would be nice to have a more `builder`-ey way to do this.
    tree = main.cell("tree", CompInst("tree", []))

    for i, ans_reg in enumerate([sum_col0, sum_col1, sum_col2, sum_col3]):
        use_tree_ports_calculated(main, f"add_col{i}", A, B, C, D, i, tree, ans_reg)

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

    main.control += [
        main.get_group("add_col0"),
        main.get_group("add_col1"),
        main.get_group("add_col2"),
        main.get_group("add_col3"),
        main.get_group("add_intermediates"),
    ]


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    add_tree(prog)
    add_main(prog)
    return prog.program


if __name__ == "__main__":
    build().emit()
