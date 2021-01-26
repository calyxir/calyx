import math
import textwrap
from prettytable import PrettyTable


def pp_block(decl, contents, indent=2):
    """Format a block like this:
        decl {
          contents
        }
    where `decl` is one line but contents can be multiple lines.
    """
    return ''.join((decl, ' {\n', textwrap.indent(contents, indent * ' '), '\n}'))


def get_pipeline_data(n, num_stages):
    """Returns the operations for each row in each stage in tuple format.
    Recall that each stage will consist of N operations, following the
    pattern:
      `a[i] +- (a[j] * phis[p])`

    From this, we can have a log2(`n`) x `n` array that stores the tuple value:
      `(i, op, m)`
      where `i` is the input index of the lhs,
            `j` is the input index of the rhs,
            and `m` is the index of the register holding the product.
    """
    operations = [[() for _ in range(n)] for _ in range(num_stages)]
    phi_index = 1
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
            phi_index += 1
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
    stage_titles = ['a'] + ['Stage {}'.format(i) for i in range(num_stages)]
    table = PrettyTable(stage_titles)
    for row in range(n):
        table_row = ['{}'.format(row)]
        for stage in range(num_stages):
            lhs, op, mult_register = operations[stage][row]
            _, rhs, phi_index = multiplies[stage][mult_register]
            table_row.append('a[{}] {} a[{}] * phis[{}]'
                             .format(lhs, op, rhs, phi_index))
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
    assert n > 0 and (not (n & (n - 1))), '{} must be a power of 2.'.format(n)
    num_stages = int(math.log2(n))
    bitwidth = num_stages + 1

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

    def pp_mul_group(stage, mul_tuple):
        mul_index, k, phi_index = mul_tuple
        mult_group_name = 's{}_mul{}'.format(stage, mul_index)
        mult_wires = [
            'mult_pipe{}.left = phi{}.out;'.format(mul_index, phi_index),
            'mult_pipe{}.right = r{}.out;'.format(mul_index, k),
            'mult_pipe{}.go = !mult_pipe{}.done ? 1\'d1;'.format(mul_index, mul_index),
            'mul{}.write_en = mult_pipe{}.done;'.format(mul_index, mul_index),
            'mul{}.in = mult_pipe{}.out;'.format(mul_index, mul_index),
            '{}[done] = mul{}.done ? 1\'d1;'.format(mult_group_name, mul_index)
        ]
        return pp_block('group {}'.format(mult_group_name), '\n'.join(mult_wires))

    def pp_op_mod_group(stage, row, operations_tuple):
        lhs, op, mul_index = operations_tuple
        comp = 'add' if op == '+' else 'sub'

        comp_index = fresh_comp_index(comp)
        mod_group_name = 's{}_r{}_op_mod'.format(stage, row)
        mod_wires = [
            '{}{}.left = r{}.out;'.format(comp, comp_index, lhs),
            '{}{}.right = mul{}.out;'.format(comp, comp_index, mul_index),
            'mod_pipe{}.left = {}{}.out;'.format(row, comp, comp_index, comp, row),
            'mod_pipe{}.right = {}\'d{};'.format(row, input_bitwidth, q),
            'mod_pipe{}.go = !mod_pipe{}.done ? 1\'d1;'.format(row, row),
            'A{}.write_en = mod_pipe{}.done;'.format(row, row),
            'A{}.in = mod_pipe{}.out;'.format(row, row),
            '{}[done] = A{}.done ? 1\'d1;'.format(mod_group_name, row)
        ]
        return pp_block('group {}'.format(mod_group_name), '\n'.join(mod_wires))

    def pp_precursor_group(row):
        group_name = 'precursor_{}'.format(row)
        wires = [
            'r{}.in = A{}.out;'.format(row, row),
            'r{}.write_en = 1\'d1;'.format(row),
            '{}[done] = r{}.done ? 1\'d1;'.format(group_name, row)
        ]
        return pp_block('group {}'.format(group_name), '\n'.join(wires))

    def pp_preamble_group(row):
        group_name = "preamble_{}".format(row)
        wires = [
            'a.addr0 = {}\'d{};'.format(bitwidth, row),
            'phis.addr0 = {}\'d{};'.format(bitwidth, row),
            'r{}.write_en = 1\'d1;'.format(row),
            'r{}.in = a.read_data;'.format(row),
            'phi{}.write_en = 1\'d1;'.format(row),
            'phi{}.in = phis.read_data;'.format(row),
            '{}[done] = r{}.done & phi{}.done? 1\'d1;'.format(group_name, row, row)
        ]
        return pp_block('group {}'.format(group_name), '\n'.join(wires))

    def pp_epilogue_group(row):
        group_name = "epilogue_{}".format(row)
        wires = [
            'a.addr0 = {}\'d{};'.format(bitwidth, row),
            'a.write_en = 1\'d1;',
            'a.write_data = A{}.out;'.format(row),
            '{}[done] = a.done ? 1\'d1;'.format(group_name)
        ]
        return pp_block('group {}'.format(group_name), '\n'.join(wires))

    def control():
        preambles = pp_block('seq', '\n'.join(['preamble_{};'.format(r) for r in range(n)]))
        epilogues = pp_block('seq', '\n'.join(['epilogue_{};'.format(r) for r in range(n)]))

        ntt_stages = []
        for s in range(num_stages):
            if s != 0:
                # Only append precursors if this is not the first stage.
                precursors = ['precursor_{};'.format(r) for r in range(n)]
                ntt_stages.append(pp_block('par', '\n'.join(precursors)))

            multiplies = ['s{}_mul{};'.format(s, i) for i in range(n // 2)]
            op_mods = ['s{}_r{}_op_mod;'.format(s, r) for r in range(n)]
            ntt_stages.append(pp_block('par', '\n'.join(multiplies)))
            ntt_stages.append(pp_block('par', '\n'.join(op_mods)))

        controls = pp_block('seq', '\n'.join((preambles, '\n'.join(ntt_stages), epilogues)))
        return pp_block('control', controls)

    def cells():
        memories = [
            'a = prim std_mem_d1({}, {}, {});'.format(input_bitwidth, n, bitwidth),
            'phis = prim std_mem_d1({}, {}, {});'.format(input_bitwidth, n, bitwidth)
        ]
        r_registers = ['r{} = prim std_reg({});'.format(row, input_bitwidth) for row in range(n)]
        A_registers = ['A{} = prim std_reg({});'.format(row, input_bitwidth) for row in range(n)]
        mul_registers = ['mul{} = prim std_reg({});'.format(i, input_bitwidth) for i in range(n // 2)]
        phi_registers = ['phi{} = prim std_reg({});'.format(row, input_bitwidth) for row in range(n)]
        mod_pipes = ['mod_pipe{} = prim std_smod_pipe({});'.format(row, input_bitwidth) for row in range(n)]
        mult_pipes = ['mult_pipe{} = prim std_smult_pipe({});'.format(i, input_bitwidth) for i in range(n // 2)]
        adds = ['add{} = prim std_sadd({});'.format(row, input_bitwidth) for row in range(n // 2)]
        subs = ['sub{} = prim std_ssub({});'.format(row, input_bitwidth) for row in range(n // 2)]

        cells = memories + r_registers + A_registers + mul_registers \
                + phi_registers + mod_pipes + mult_pipes + adds + subs
        return pp_block('cells', '\n'.join(cells))

    def wires():
        preamble_groups = [pp_preamble_group(r) for r in range(n)]
        precursor_groups = [pp_precursor_group(r) for r in range(n)]
        mul_groups = [pp_mul_group(s, multiplies[s][i]) for s in range(num_stages) for i in range(n // 2)]
        op_mod_groups = [pp_op_mod_group(s, r, operations[s][r]) for s in range(num_stages) for r in range(n)]
        epilogue_groups = [pp_epilogue_group(r) for r in range(n)]
        groups = preamble_groups + precursor_groups + mul_groups + op_mod_groups + epilogue_groups
        return pp_block('wires', '\n'.join(groups))

    pp_table(operations, multiplies, n, num_stages)
    print('import "primitives/std.lib";')
    print(
        pp_block('component main() -> ()', '\n'.join(
            (
                cells(), wires(), control()
            )
        ))
    )


if __name__ == '__main__':
    import sys, argparse, json

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
