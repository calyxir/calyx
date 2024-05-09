#!/usr/bin/env python3

from prettytable import PrettyTable
import numpy as np
import calyx.py_ast as ast
import calyx.builder as cb
from calyx.utils import bits_needed


def reduce_parallel_control_pass(component: ast.Component, N: int, input_size: int):
    """Reduces the amount of fan-out by reducing
    parallelization in the execution flow
    by a factor of `N`.

    For example, given an input size 4 and
    reduction factor N = 2:

    Before:
    par { s0_mul0; s0_mul1; }
    par { s0_r0_op_mod; s0_r1_op_mod; s0_r2_op_mod; s0_r3_op_mod; }
    ...

    After:
    par { s0_mul0; s0_mul1; }
    par { s0_r0_op_mod; s0_r1_op_mod; }
    par { s0_r2_op_mod; s0_r3_op_mod; }
    ...
    """
    assert (
        N and 0 < N < input_size and (not (N & (N - 1)))
    ), f"""N: {N} should be a power of two within bounds (0, {input_size})."""

    reduced_controls = []
    for control in component.controls.stmts:
        if not isinstance(control, ast.ParComp):
            reduced_controls.append(control)
            continue

        enable = next(iter(control.stmts), None).stmt
        # Parallelized multiplies are already a factor of 1/2 less.
        factor = N // 2 if "mul" in enable else N

        reduced_controls.extend(
            ast.ParComp(x) for x in np.split(np.array(control.stmts), factor)
        )

    component.controls = ast.SeqComp(reduced_controls)


def get_pipeline_data(n, num_stages):
    """Returns the operations for each row in each stage in tuple format.
    Recall that each stage will consist of N operations, following the
    pattern:
      `a[i] +- (a[j] * phis[p])`

    From this, we can have a log2(`n`) x `n` array that stores the tuple value:
      `(i, op, m)`
      where `i` is the input index of the lhs,
            `op` is the operation conducted, in {+, -},
            and `m` is the index of the register holding the product.
    """
    operations = [[() for _ in range(n)] for _ in range(num_stages)]
    t = n
    for i in range(0, num_stages):
        t >>= 1
        j = t
        mult_register = 0
        while j < n:
            for k in range(j, j + t):
                operations[i][k - t] = (k - t, "+", mult_register)
                operations[i][k] = (k - t, "-", mult_register)
                mult_register += 1
            j += t << 1
    return operations


def get_multiply_data(n, num_stages):
    """Returns a tuple value for each unique product that contains:
       (register index, input index, phi index)

    Since each stage in the pipeline has at most N / 2 unique products
    calculated, we want to only use N / 2 `mult_pipeline`s in each stage. This
    requires us to map which products go with which calculation.

    This calculation is usually referred to as `V`, and would look something
    like:
      `V = a[x] * phis[y]`
    and used to calculate:
      `a[k]   = U + V mod q`
      `a[k+t] = U - V mod q`
    """
    mults = [[] for _ in range(num_stages)]
    phi_index = 1
    t = n
    for i in range(0, num_stages):
        t >>= 1
        j = t
        register_index = 0
        while j < n:
            for input_index in range(j, j + t):
                mults[i].append((register_index, input_index, phi_index))
                register_index += 1
            j += t << 1
            phi_index += 1
    return mults


def pp_table(operations, multiplies, n, num_stages):
    """Pretty prints a table describing the
    calculations made during the pipeline."""
    stage_titles = ["a"] + [f"Stage {i}" for i in range(num_stages)]
    table = PrettyTable(stage_titles)
    for row in range(n):
        table_row = [f"{row}"]
        for stage in range(num_stages):
            lhs, op, mult_register = operations[stage][row]
            _, rhs, phi_index = multiplies[stage][mult_register]
            table_row.append(f"a[{lhs}] {op} a[{rhs}] * phis[{phi_index}]")
        table.add_row(table_row)
    table = table.get_string().split("\n")
    table = [" ".join(("//", line)) for line in table]
    print("\n".join(table))


def generate_ntt_pipeline(input_bitwidth: int, n: int, q: int):
    """
    Prints a pipeline in Calyx for the cooley-tukey algorithm
    that uses phis in bit-reversed order.

    `n`:
      Length of the input array.
    `input_bitwidth`:
      Bit width of the values in the input array.
    `q`:
      The modulus value.

    Reference:
    https://www.microsoft.com/en-us/research/wp-content/uploads/2016/05/RLWE-1.pdf
    """
    assert n > 0 and (not (n & (n - 1))), f"Input length: {n} must be a power of 2."
    bitwidth = bits_needed(n)
    num_stages = bitwidth - 1

    operations = get_pipeline_data(n, num_stages)
    multiplies = get_multiply_data(n, num_stages)

    # Used to determine the index of the component
    # for the `sadd` and `ssub` primitives.
    component_counts = {"add": 0, "sub": 0}

    def fresh_comp_index(op):
        # Produces a new index for the component used in the stage.
        # This allows for N / 2 `sadd` and `ssub` components.

        saved_count = component_counts[op]
        if component_counts[op] == (n // 2) - 1:
            # Reset for the next stage.
            component_counts[op] = 0
        else:
            component_counts[op] += 1

        return saved_count

    # Memory component variables.
    # input = CompVar("a")
    # phis = CompVar("phis")

    def mul_group(comp: cb.ComponentBuilder, stage, mul_tuple):
        mul_index, k, phi_index = mul_tuple
        comp.binary_use_names(
            f"mult_pipe{mul_index}",
            f"phi{phi_index}",
            f"r{k}",
            f"s{stage}_mul{mul_index}",
        )

    def op_mod_group(comp: cb.ComponentBuilder, stage, row, operations_tuple):
        lhs, op, mul_index = operations_tuple
        op_name = "add" if op == "+" else "sub"
        comp_index = fresh_comp_index(op_name)

        op = comp.get_cell(f"{op_name}{comp_index}")
        reg = comp.get_cell(f"r{lhs}")
        mul = comp.get_cell(f"mult_pipe{mul_index}")
        mod_pipe = comp.get_cell(f"mod_pipe{row}")
        A = comp.get_cell(f"A{row}")
        with comp.group(f"s{stage}_r{row}_op_mod") as g:
            op.left = reg.out
            op.right = mul.out
            mod_pipe.left = op.out
            mod_pipe.right = q
            mod_pipe.go = ~mod_pipe.done @ 1
            A.write_en = mod_pipe.done
            A.in_ = mod_pipe.out_remainder
            g.done = A.done

    def precursor_group(comp: cb.ComponentBuilder, row):
        r = comp.get_cell(f"r{row}")
        A = comp.get_cell(f"A{row}")
        comp.reg_store(r, A.out, f"precursor_{row}")

    def preamble_group(comp: cb.ComponentBuilder, row):
        reg = comp.get_cell(f"r{row}")
        phi = comp.get_cell(f"phi{row}")
        input = comp.get_cell("a")
        phis = comp.get_cell("phis")
        with main.group(f"preamble_{row}") as preamble:
            input.addr0 = row
            phis.addr0 = row
            reg.write_en = 1
            reg.in_ = input.read_data
            phi.write_en = 1
            phi.in_ = phis.read_data
            preamble.done = (reg.done & phi.done) @ 1

    def epilogue_group(comp: cb.ComponentBuilder, row):
        input = comp.get_cell("a")
        A = comp.get_cell(f"A{row}")
        comp.mem_store_comb_mem_d1(input, row, A.out, f"epilogue_{row}")

    def insert_cells(comp: cb.ComponentBuilder):
        # memories
        comp.comb_mem_d1("a", input_bitwidth, n, bitwidth, is_external=True)
        comp.comb_mem_d1("phis", input_bitwidth, n, bitwidth, is_external=True)

        for r in range(n):
            comp.reg(input_bitwidth, f"r{r}")  # r_regs
            comp.reg(input_bitwidth, f"A{r}")  # A_regs
            comp.reg(input_bitwidth, f"phi{r}")  # phi_regs
            comp.div_pipe(input_bitwidth, f"mod_pipe{r}", signed=True)  # mod_pipes

        for i in range(n // 2):
            comp.reg(input_bitwidth, f"mult{i}")  # mul_regs
            comp.mult_pipe(input_bitwidth, f"mult_pipe{i}", signed=True)  # mult_pipes
            comp.add(input_bitwidth, f"add{i}", signed=True)  # adds
            comp.sub(input_bitwidth, f"sub{i}", signed=True)  # subs

    def wires(main: cb.ComponentBuilder):
        for r in range(n):
            preamble_group(main, r)
        for r in range(n):
            precursor_group(main, r)
        for s in range(num_stages):
            for i in range(n // 2):
                mul_group(main, s, multiplies[s][i])
        for s in range(num_stages):
            for r in range(n):
                op_mod_group(main, s, r, operations[s][r])
        for r in range(n):
            epilogue_group(main, r)

    def control():
        preambles = [ast.SeqComp([ast.Enable(f"preamble_{r}") for r in range(n)])]
        epilogues = [ast.SeqComp([ast.Enable(f"epilogue_{r}") for r in range(n)])]

        ntt_stages = []
        for s in range(num_stages):
            if s != 0:
                # Only append precursors if this is not the first stage.
                ntt_stages.append(
                    ast.ParComp([ast.Enable(f"precursor_{r}") for r in range(n)])
                )
            # Multiply
            ntt_stages.append(
                ast.ParComp([ast.Enable(f"s{s}_mul{i}") for i in range(n // 2)])
            )
            # Addition or subtraction mod `q`
            ntt_stages.append(
                ast.ParComp([ast.Enable(f"s{s}_r{r}_op_mod") for r in range(n)])
            )
        return ast.SeqComp(preambles + ntt_stages + epilogues)

    pp_table(operations, multiplies, n, num_stages)
    prog = cb.Builder()
    main = prog.component("main")
    insert_cells(main)
    wires(main)
    main.component.controls = control()
    return prog.program


if __name__ == "__main__":
    import argparse
    import json

    parser = argparse.ArgumentParser(description="NTT")
    parser.add_argument("file", nargs="?", type=str)
    parser.add_argument("-b", "--input_bitwidth", type=int)
    parser.add_argument("-n", "--input_size", type=int)
    parser.add_argument("-q", "--modulus", type=int)
    parser.add_argument("-par_red", "--parallel_reduction", type=int)

    args = parser.parse_args()

    input_bitwidth, input_size, modulus = None, None, None
    required_fields = [args.input_bitwidth, args.input_size, args.modulus]
    if all(map(lambda x: x is not None, required_fields)):
        input_bitwidth = args.input_bitwidth
        input_size = args.input_size
        modulus = args.modulus
        parallel_reduction = args.parallel_reduction
    elif args.file is not None:
        with open(args.file, "r") as f:
            spec = json.load(f)
            input_bitwidth = spec["input_bitwidth"]
            input_size = spec["input_size"]
            modulus = spec["modulus"]
            parallel_reduction = spec.get("parallel_reduction")
    else:
        parser.error(
            "Need to pass either `-f FILE` or all of `-b INPUT_BITWIDTH -n INPUT_SIZE -q MODULUS`"
        )

    program = generate_ntt_pipeline(input_bitwidth, input_size, modulus)

    if parallel_reduction is not None:
        for c in program.components:
            reduce_parallel_control_pass(c, parallel_reduction, input_size)

    program.emit()
