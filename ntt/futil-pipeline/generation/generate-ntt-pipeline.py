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


def generate_pipeline_operations(n, num_stages):
    """Returns the operations for each row in each stage in tuple format.
    Recall that each stage will consist of N operations, following the
    pattern:
      `a[i] +- (a[j] * phis[p])`

    From this, we can have a log2(`n`) x `n` array that stores the tuple value:
      `(i, op, j, p)`
      where `i` is the input index of the lhs,
            `j` is the input index of the rhs,
            `op` is the operation (either + or -),
            and `p` is the index within phis.
    """
    structure = [[() for _ in range(n)] for _ in range(num_stages)]
    phi_index = 1
    t = n
    for i in range(0, num_stages):
        t >>= 1
        j = t
        while j < n:
            for k in range(j, j + t):
                structure[i][k - t] = ((k - t, '+', k, phi_index))
                structure[i][k] = ((k - t, '-', k, phi_index))
            j += t << 1
            phi_index += 1
    return structure


def pp_table(structure, n, num_stages):
    """Pretty prints a table describing the calculates made during the pipeline."""
    stage_titles = ['a'] + ['Stage {}'.format(i) for i in range(num_stages)]
    table = PrettyTable(stage_titles)
    for row in range(n):
        table_row = ['{}'.format(row)]
        for stage in range(num_stages):
            lhs, op, rhs, phi_idx = structure[stage][row]
            table_row.append('a[{}] {} a[{}] * phis[{}]'
                             .format(lhs, op, rhs, phi_idx))
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

    structure = generate_pipeline_operations(n, num_stages)

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

    def pp_stage_groups(stage, row, structure_tuple):
        lhs, op, rhs, phi_index = structure_tuple
        comp = 'add' if op == '+' else 'sub'

        mult_group_name = 's{}_r{}_mul'.format(stage, row)
        mult_wires = [
            'mult_pipe{}.left = phi{}.out;'.format(row, phi_index),
            'mult_pipe{}.right = r{}.out;'.format(row, rhs),
            'mult_pipe{}.go = !mult_pipe{}.done ? 1\'d1;'.format(row, row),
            'mul{}.write_en = mult_pipe{}.done;'.format(row, row),
            'mul{}.in = mult_pipe{}.out;'.format(row, row),
            '{}[done] = mul{}.done ? 1\'d1;'.format(mult_group_name, row)
        ]
        mult_group = pp_block('group {}'.format(mult_group_name), '\n'.join(mult_wires))

        comp_index = fresh_comp_index(comp)
        mod_group_name = 's{}_r{}_mod'.format(stage, row)
        mod_wires = [
            '{}{}.left = r{}.out;'.format(comp, comp_index, lhs),
            '{}{}.right = mul{}.out;'.format(comp, comp_index, row, row),
            'mod_pipe{}.left = {}{}.out;'.format(row, comp, comp_index, comp, row),
            'mod_pipe{}.right = {}\'d{};'.format(row, input_bitwidth, q),
            'mod_pipe{}.go = !mod_pipe{}.done ? 1\'d1;'.format(row, row),
            'A{}.write_en = mod_pipe{}.done;'.format(row, row),
            'A{}.in = mod_pipe{}.out;'.format(row, row),
            '{}[done] = A{}.done ? 1\'d1;'.format(mod_group_name, row)
        ]
        mod_group = pp_block('group {}'.format(mod_group_name), '\n'.join(mod_wires))

        return '\n'.join((mult_group, mod_group))

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

            rows = ['seq {{ s{}_r{}_mul;'.format(s, r) + ' s{}_r{}_mod; }}'.format(s, r) for r in range(n)]
            ntt_stages.append(pp_block('par', '\n'.join(rows)))

        controls = pp_block('seq', '\n'.join((preambles, '\n'.join(ntt_stages), epilogues)))
        return pp_block('control', controls)

    def cells():
        memories = [
            'a = prim std_mem_d1_ext({}, {}, {});'.format(input_bitwidth, n, bitwidth),
            'phis = prim std_mem_d1_ext({}, {}, {});'.format(input_bitwidth, n, bitwidth)
        ]
        r_registers = ['r{} = prim std_reg({});'.format(row, input_bitwidth) for row in range(n)]
        A_registers = ['A{} = prim std_reg({});'.format(row, input_bitwidth) for row in range(n)]
        mul_registers = ['mul{} = prim std_reg({});'.format(row, input_bitwidth) for row in range(n)]
        phi_registers = ['phi{} = prim std_reg({});'.format(row, input_bitwidth) for row in range(n)]
        mod_pipes = ['mod_pipe{} = prim std_smod_pipe({});'.format(row, input_bitwidth) for row in range(n)]
        mult_pipes = ['mult_pipe{} = prim std_smult_pipe({});'.format(row, input_bitwidth) for row in range(n)]
        adds = ['add{} = prim std_sadd({});'.format(row, input_bitwidth) for row in range(n // 2)]
        subs = ['sub{} = prim std_ssub({});'.format(row, input_bitwidth) for row in range(n // 2)]

        cells = memories + r_registers + A_registers + mul_registers \
                + phi_registers + mod_pipes + mult_pipes + adds + subs
        return pp_block('cells', '\n'.join(cells))

    def wires():
        preamble_groups = [pp_preamble_group(r) for r in range(n)]
        precursor_groups = [pp_precursor_group(r) for r in range(n)]
        stage_groups = [pp_stage_groups(s, r, structure[s][r]) for s in range(num_stages) for r in range(n)]
        epilogue_groups = [pp_epilogue_group(r) for r in range(n)]
        groups = preamble_groups + precursor_groups + stage_groups + epilogue_groups
        return pp_block('wires', '\n'.join(groups))

    pp_table(structure, n, num_stages)
    print('import "primitives/std.lib";')
    print(
        pp_block('component main() -> ()', '\n'.join(
            (
                cells(), wires(), control()
            )
        ))
    )


if __name__ == '__main__':
    """
    Expects a file in the following format:
      ```
      input_bitwidth: <input0>
      n: <input1>
      q: <input2>
      ```
    """
    import sys

    inputs = [int(''.join(filter(str.isdigit, line))) for line in sys.stdin]
    generate_ntt_pipeline(input_bitwidth=inputs[0], n=inputs[1], q=inputs[2])
