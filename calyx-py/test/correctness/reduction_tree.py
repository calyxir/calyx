# pylint: disable=import-error
from typing import List
import calyx.builder as cb


def add_tree(prog):
    """Inserts the component `tree` into the program.
    It has:
    - four inputs, `leaf0`, `leaf1`, `leaf2`, and `leaf3`
    - one output, `sum`

    When done, it puts the sum of the four leaves into `sum`.
    Returns a handle to the component `tree`.
    """

    tree: cb.ComponentBuilder = prog.component("tree")
    # Add the four inputs ports, and stash handles to those inputs.
    [leaf0, leaf1, leaf2, leaf3] = [tree.input(f"leaf{i}", 32) for i in range(4)]

    # Add the output port.
    tree.output("sum", 32)

    # Into the component `tree`, add the wiring for three adder groups that will
    # use the tree to perform their additions.
    # These need to be orchestrated in the control below.
    add_l0_l1, left = tree.add_store_in_reg(leaf0, leaf1)
    add_l2_l3, right = tree.add_store_in_reg(leaf2, leaf3)
    add_l_r_nodes, root = tree.add_store_in_reg(left.out, right.out)

    # Continuously output the value of the root register.
    # It is the invoker's responsibility to ensure that the tree is done
    # before reading this value.
    with tree.continuous:
        tree.this().sum = root.out

    tree.control += [cb.par(add_l0_l1, add_l2_l3), add_l_r_nodes]
    return tree


def use_tree_ports_provided(comp, group, port0, port1, port2, port3, tree, ans_mem):
    """Orchestrates the use of the component `tree`.
    Adds wiring for a new group called {group}.
    In {group}, assumes that `tree` exists and is set up as above.
    Puts into the tree's four leaves the values port0, port1, port2, and port3.
    Runs the tree, waits for the tree to be done, and stores the answer in {ans_mem}.
    Finally, returns a handle to {group}.
    """

    with comp.group(group) as tree_use:
        tree.leaf0 = port0
        tree.leaf1 = port1
        tree.leaf2 = port2
        tree.leaf3 = port3
        tree.go = ~tree.done @ cb.HI
        ans_mem.addr0 = tree.done @ 0
        ans_mem.write_data = tree.done @ tree.sum
        ans_mem.write_en = tree.done @ 1
        tree_use.done = ans_mem.done
    return tree_use


def use_tree_ports_calculated(
    comp, group, mem_a, mem_b, mem_c, mem_d, i, tree, ans_reg
):
    """Orchestrates the use of the component `tree`.
    Adds wiring for a new group called {group}.
    In {group}, assumes that `tree` exists and is set up as above.
    Puts into the tree's four leaves the values the values a[i], b[i], c[i], and d[i].
    Runs the tree, waits for the tree to be done, and stores the answer in {ans_reg}.
    Finally, returns a handle to {group}.
    """
    # i.e., much like the above, but instead of getting the ports as arguments,
    # it must first calculate them.
    # It also stores the answer in a register, rather than a memory.

    with comp.group(group) as tree_use:
        mem_a.addr0 = mem_b.addr0 = mem_c.addr0 = mem_d.addr0 = i
        tree.leaf0 = mem_a.read_data
        tree.leaf1 = mem_b.read_data
        tree.leaf2 = mem_c.read_data
        tree.leaf3 = mem_d.read_data
        tree.go = ~tree.done @ cb.HI
        ans_reg.write_en = tree.done @ 1
        ans_reg.in_ = tree.done @ tree.sum
        tree_use.done = ans_reg.done
    return tree_use


def add_main(prog, tree):
    """Inserts the component `main` into the program.
    It requires:
    - Memories `A`, `B`, `C`, `D`, of length 4 each, to be driven from the data file.
    - A memory `ans` to store the result, also driven from the data file.

    It puts the sum of elements of the four memory cells into `ans`.
    """
    main = prog.component("main")
    # Four memories, each of length 4.
    [mem_a, mem_b, mem_c, mem_d] = [
        main.comb_mem_d1(name, 32, 4, 32, is_external=True)
        for name in ["A", "B", "C", "D"]
    ]
    mem_ans = main.comb_mem_d1("ans", 32, 1, 1, is_external=True)
    # Four answer registers.
    [sum_col0, sum_col1, sum_col2, sum_col3] = [main.reg(32) for i in range(4)]
    tree = main.cell("tree", tree)

    adder_groups: List[cb.GroupBuilder] = [
        # Fill each of our answer registers will the sum of the corresponding column.
        # The tree will be used four times, once for each column.
        # Each time, we will receive a handle to the group that does the work.
        # We stash these groups in `adder_groups`.
        use_tree_ports_calculated(
            main, f"add_col{i}", mem_a, mem_b, mem_c, mem_d, i, tree, ans_reg
        )
        for i, ans_reg in enumerate([sum_col0, sum_col1, sum_col2, sum_col3])
    ]

    adder_groups.append(
        # We also create and stash a further group:
        # this one does the work of adding the four columns.
        use_tree_ports_provided(
            main,
            "add_intermediates",
            sum_col0.out,
            sum_col1.out,
            sum_col2.out,
            sum_col3.out,
            tree,
            mem_ans,
        )
    )

    # Control is straightforward: just run all the groups in `adder_groups` in sequence.
    main.control += adder_groups


def build():
    """Top-level function to build the program."""
    prog = cb.Builder()
    tree = add_tree(prog)
    add_main(prog, tree)
    return prog.program


if __name__ == "__main__":
    build().emit()
