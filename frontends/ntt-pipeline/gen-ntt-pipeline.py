from prettytable import PrettyTable

from futil.ast import *
from futil.utils import bits_needed


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
                operations[i][k - t] = ((k - t, '+', mult_register))
                operations[i][k] = ((k - t, '-', mult_register))
                mult_register += 1
            j += t << 1
    return operations


def get_multiply_data(n, num_stages):
    """Returns a tuple value for each unique product that contains:
       (register index, input index, phi index)

    Since each stage in the pipeline has at most N / 2 unique products calculated,
    we want to only use N / 2 `mult_pipeline`s in each stage. This requires us to
    map which products go with which calculation.

    This calculation is usually referred to as `V`, and would look something like:
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
    """Pretty prints a table describing the calculations made during the pipeline."""
    stage_titles = ['a'] + [f'Stage {i}' for i in range(num_stages)]
    table = PrettyTable(stage_titles)
    for row in range(n):
        table_row = [f'{row}']
        for stage in range(num_stages):
            lhs, op, mult_register = operations[stage][row]
            _, rhs, phi_index = multiplies[stage][mult_register]
            table_row.append(f'a[{lhs}] {op} a[{rhs}] * phis[{phi_index}]')
        table.add_row(table_row)
    table = table.get_string().split("\n")
    table = [' '.join(('//', line)) for line in table]
    print('\n'.join(table))


def generate_ntt_pipeline(input_bitwidth, n, q):
    """
    Prints a pipeline in FuTIL for the cooley-tukey algorithm
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
    assert n > 0 and (not (n & (n - 1))), f'Input length: {n} must be a power of 2.'
    bitwidth = bits_needed(n)
    num_stages = bitwidth - 1

    operations = get_pipeline_data(n, num_stages)
    multiplies = get_multiply_data(n, num_stages)

    # Used to determine the index of the component
    # for the `sadd` and `ssub` primitives.
    component_counts = {'add': 0, 'sub': 0}

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
    input = CompVar('a')
    phis = CompVar('phis')

    def mul_group(stage, mul_tuple):
        mul_index, k, phi_index = mul_tuple

        group_name = CompVar(f's{stage}_mul{mul_index}')
        mult_pipe = CompVar(f'mult_pipe{mul_index}')
        mul = CompVar(f'mul{mul_index}')
        phi = CompVar(f'phi{phi_index}')
        reg = CompVar(f'r{k}')
        connections = [
            Connect(CompPort(phi, 'out'), CompPort(mult_pipe, 'left')),
            Connect(CompPort(reg, 'out'), CompPort(mult_pipe, 'right')),
            Connect(ConstantPort(1, 1), CompPort(mult_pipe, 'go'), Not(Atom(CompPort(mult_pipe, 'done')))),
            Connect(CompPort(mult_pipe, 'done'), CompPort(mul, 'write_en')),
            Connect(CompPort(mult_pipe, 'out'), CompPort(mul, 'in')),
            Connect(CompPort(mul, 'done'), HolePort(group_name, 'done'))
        ]
        return Group(group_name, connections)

    def op_mod_group(stage, row, operations_tuple):
        lhs, op, mul_index = operations_tuple
        comp = 'add' if op == '+' else 'sub'
        comp_index = fresh_comp_index(comp)

        group_name = CompVar(f's{stage}_r{row}_op_mod')
        op = CompVar(f'{comp}{comp_index}')
        reg = CompVar(f'r{lhs}')
        mul = CompVar(f'mul{mul_index}')
        mod_pipe = CompVar(f'mod_pipe{row}')
        A = CompVar(f'A{row}')
        connections = [
            Connect(CompPort(reg, 'out'), CompPort(op, 'left')),
            Connect(CompPort(mul, 'out'), CompPort(op, 'right')),
            Connect(CompPort(op, 'out'), CompPort(mod_pipe, 'left')),
            Connect(ConstantPort(input_bitwidth, q), CompPort(mod_pipe, 'right')),
            Connect(ConstantPort(1, 1), CompPort(mod_pipe, 'go'), Not(Atom(CompPort(mod_pipe, 'done')))),
            Connect(CompPort(mod_pipe, 'done'), CompPort(A, 'write_en')),
            Connect(CompPort(mod_pipe, 'out'), CompPort(A, 'in')),
            Connect(CompPort(A, 'done'), HolePort(group_name, 'done'))
        ]
        return Group(group_name, connections)

    def precursor_group(row):
        group_name = CompVar(f'precursor_{row}')
        r = CompVar(f'r{row}')
        A = CompVar(f'A{row}')
        connections = [
            Connect(CompPort(A, 'out'), CompPort(r, 'in')),
            Connect(ConstantPort(1, 1), CompPort(r, 'write_en')),
            Connect(CompPort(r, 'done'), HolePort(group_name, 'done'))
        ]
        return Group(group_name, connections)

    def preamble_group(row):
        reg = CompVar(f'r{row}')
        phi = CompVar(f'phi{row}')
        group_name = CompVar(f'preamble_{row}')
        connections = [
            Connect(ConstantPort(bitwidth, row), CompPort(input, 'addr0')),
            Connect(ConstantPort(bitwidth, row), CompPort(phis, 'addr0')),
            Connect(ConstantPort(1, 1), CompPort(reg, 'write_en')),
            Connect(CompPort(input, 'read_data'), CompPort(reg, 'in')),
            Connect(ConstantPort(1, 1), CompPort(phi, 'write_en')),
            Connect(CompPort(phis, 'read_data'), CompPort(phi, 'in')),
            Connect(ConstantPort(1, 1), HolePort(group_name, 'done'),
                    And(Atom(CompPort(reg, 'done')),
                        Atom(CompPort(phi, 'done'))))
        ]
        return Group(group_name, connections)

    def epilogue_group(row):
        group_name = CompVar(f'epilogue_{row}')
        A = CompVar(f'A{row}')
        connections = [
            Connect(ConstantPort(bitwidth, row), CompPort(input, 'addr0')),
            Connect(ConstantPort(1, 1), CompPort(input, 'write_en')),
            Connect(CompPort(A, 'out'), CompPort(input, 'write_data')),
            Connect(CompPort(input, 'done'), HolePort(group_name, 'done'))
        ]
        return Group(group_name, connections)

    def cells():
        stdlib = Stdlib()

        memories = [
            Cell(input, stdlib.mem_d1(input_bitwidth, n, bitwidth)),
            Cell(phis, stdlib.mem_d1(input_bitwidth, n, bitwidth))
        ]
        r_regs = [Cell(CompVar(f'r{r}'), stdlib.register(input_bitwidth)) for r in range(n)]
        A_regs = [Cell(CompVar(f'A{r}'), stdlib.register(input_bitwidth)) for r in range(n)]
        mul_regs = [Cell(CompVar(f'mul{i}'), stdlib.register(input_bitwidth)) for i in range(n // 2)]
        phi_regs = [Cell(CompVar(f'phi{r}'), stdlib.register(input_bitwidth)) for r in range(n)]
        mod_pipes = [
            Cell(
                CompVar(f'mod_pipe{r}'),
                stdlib.op('mod_pipe', input_bitwidth, signed=True)
            ) for r in range(n)
        ]
        mult_pipes = [
            Cell(
                CompVar(f'mult_pipe{i}'),
                stdlib.op('mult_pipe', input_bitwidth, signed=True)
            ) for i in range(n // 2)
        ]
        adds = [
            Cell(
                CompVar(f'add{i}'),
                stdlib.op('add', input_bitwidth, signed=True)
            ) for i in range(n // 2)
        ]
        subs = [
            Cell(
                CompVar(f'sub{i}'),
                stdlib.op('sub', input_bitwidth, signed=True)
            ) for i in range(n // 2)
        ]

        return memories + r_regs + A_regs + mul_regs + phi_regs + mod_pipes + mult_pipes + adds + subs

    def wires():
        preambles = [preamble_group(r) for r in range(n)]
        precursors = [precursor_group(r) for r in range(n)]
        muls = [mul_group(s, multiplies[s][i]) for s in range(num_stages) for i in range(n // 2)]
        op_mods = [op_mod_group(s, r, operations[s][r]) for s in range(num_stages) for r in range(n)]
        epilogues = [epilogue_group(r) for r in range(n)]
        return preambles + precursors + muls + op_mods + epilogues

    def control():
        preambles = [SeqComp([Enable(f'preamble_{r}') for r in range(n)])]
        epilogues = [SeqComp([Enable(f'epilogue_{r}') for r in range(n)])]

        ntt_stages = []
        for s in range(num_stages):
            if s != 0:
                # Only append precursors if this is not the first stage.
                ntt_stages.append(ParComp([Enable(f'precursor_{r}') for r in range(n)]))
            # Multiply
            ntt_stages.append(ParComp([Enable(f's{s}_mul{i}') for i in range(n // 2)]))
            # Addition or subtraction mod `q`
            ntt_stages.append(ParComp([Enable(f's{s}_r{r}_op_mod') for r in range(n)]))
        return ControlEntry(ControlEntryType.Seq, preambles + ntt_stages + epilogues)

    pp_table(operations, multiplies, n, num_stages)
    Program(
        imports=[Import('primitives/std.lib')],
        components=[
            Component(
                'main',
                inputs=[],
                outputs=[],
                structs=cells() + wires(),
                controls=control()
            )
        ]
    ).emit()


if __name__ == '__main__':
    import argparse, json

    parser = argparse.ArgumentParser(description='NTT')
    parser.add_argument('file', nargs='?', type=str)
    parser.add_argument('-b', '--input_bitwidth', type=int)
    parser.add_argument('-n', '--input_size', type=int)
    parser.add_argument('-q', '--modulus', type=int)

    args = parser.parse_args()

    input_bitwidth, input_size, modulus = None, None, None

    fields = [args.input_bitwidth, args.input_size, args.modulus]
    if all(map(lambda x: x is not None, fields)):
        input_bitwidth = args.input_bitwidth
        input_size = args.input_size
        modulus = args.modulus
    elif args.file is not None:
        with open(args.file, 'r') as f:
            spec = json.load(f)
            input_bitwidth = spec['input_bitwidth']
            input_size = spec['input_size']
            modulus = spec['modulus']
    else:
        parser.error("Need to pass either `-f FILE` or all of `-b INPUT_BITWIDTH -n INPUT_SIZE -q MODULUS`")

    generate_ntt_pipeline(input_bitwidth, input_size, modulus)
