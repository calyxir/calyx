import subprocess
import os

from futil.ast import *
from futil.utils import block
from relay_utils import *
from typing import List
from tempfile import NamedTemporaryFile, TemporaryFile


IMPORT_STATEMENT = """import "primitives/std.lib";\n"""
NO_ERR = "2>/dev/null"
NEWL = '\n'
CHARACTER_I = chr(ord('i'))  # Starting index variable name for Dahlia array iteration.


def next_character(ch, dir=1):
    """
    Returns the next character after 'ch'.
    If `dir` is positive, then will return 'ch' + 1. Otherwise, it will return 'ch' - 1.
    """
    return chr(ord(ch) + 1) if dir > 0 else chr(ord(ch) - 1)


def pp_dahlia_parameters(fd: DahliaFuncDef):
    # TODO: Update documentation. Simplify.
    """
    Pretty print for Dahlia memory declarations, e.g.
    `decl X: ubit<32> [1][10];`
    """

    cells = []
    for cell in fd.args + [fd.dest]:
        cell_str = f'{cell.id.name}: {fd.data_type}'
        for i in range(0, get_dims(cell.comp)):
            cell_str += f'[{cell.comp.args[i + 1]}]'
        cells.append(cell_str)
    return cells


def pp_dahlia_function_definition(fd: DahliaFuncDef, decls: List[str], body: str):
    return block(
        f'def {fd.function_id}({", ".join(decls)}) =',
        body,
        sep=''
    )


def pp_dahlia_loop(fd: DahliaFuncDef, body: str, num_dimensions, data=None):
    # TODO: Update documentation. Simplify.
    """
    Returns an iteration over data with `body` as the work done within the nested loop(s).
    Many tensor functions share the same control flow: (1) Iterate `num_dimensions` times, and (2) do some work in body.
    For example, if `data` is a 2D primitive of size (M, N) and body == `X;`, then this will return:

    ```
    for (let i: ubit<X> = 0..M) {
      for (let j: ubit<Y> = 0..N) {
        X;
      }
    }
    ```

    Notes:
    If `data` is provided, it will be used to determine the `num_dimensions` as well as the corresponding bitwidths
    and memory sizes. This occurs only in special cases; otherwise, the `output` of the `relay_function` will
    determine these.
    """
    variable_name = CHARACTER_I
    program = []
    SPACING = ''
    output = fd.dest if data == None else data
    for i in range(0, num_dimensions):
        size, index_size = output.comp.args[i + 1], output.comp.args[i + num_dimensions + 1]
        program.append(f'{SPACING}for (let {variable_name}: ubit<{index_size}> = 0..{size}) {{')
        variable_name = next_character(variable_name)
        SPACING += '  '
    program.append(f'{SPACING}{body}')

    for i in range(0, num_dimensions):
        SPACING = SPACING[:-2]
        program.append(SPACING + '}')
    return '\n'.join(program)


def dahlia_to_futil(dahlia_definitions: str):
    """
    Takes in a string representation of a Dahlia program, lowers it to FuTIL with the given `component_name`,
    and applies the `externalize` pass. This pass exposes the inputs and outputs of primitive types that are
    declared external, e.g. `@external(1) std_mem_d1`, and places them in the inputs and outputs of the respective component.

    Example:
        ------ Dahlia, component name: ProcessX ------
        import "foo.h" { ... }
        decl X: ubit<32>[4];
        ...

        ------------- Lower to FuTIL -----------------
        component ProcessX() -> () {
          X = prim std_mem_d1(32, 4, 2);
          ...
        }

        TODO: Update documentation.
    """

    with NamedTemporaryFile() as tf0, NamedTemporaryFile() as tf1:
        tf0.write(bytes(dahlia_definitions, 'UTF-8'))
        tf0.seek(0), tf1.seek(0)
        fuse_binary = os.environ['DAHLIA_EXEC'] if 'DAHLIA_EXEC' in os.environ else 'fuse'
        command = f"""{fuse_binary} {tf0.name} --lower -b=futil > {tf1.name} -l=error"""
        subprocess.Popen(command, stdout=subprocess.PIPE, shell=True).communicate()

        components = tf1.read().decode()
        # Don't add the git hash or double-import the primitives library.
        begin = components.find('component')
        # Don't import the empty main component.
        end = components.find('component main() -> () {')
        return components[begin:end]
