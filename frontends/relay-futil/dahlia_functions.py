import subprocess
import os

from tempfile import NamedTemporaryFile, TemporaryFile
from futil_ast import *

IMPORT_STATEMENT = """import "primitives/std.lib";\n"""
NO_ERR = "2>/dev/null"


def lower_dahlia_program(prog, component_name):
    '''
    Takes in a string representation of a Dahlia program, lowers it to FuTIL with the given `component_name`,
    and applies the `externalize` pass. This pass exposes the inputs and outputs of primitive types that are
    declared external, e.g. `std_mem_d1_ext`, and places them in the inputs and outputs of the respective component.

    Example:
        ------ Dahlia, component name: ProcessX ------
        decl X: ubit<32>[4];
        ...

        ------------- Lower to FuTIL -----------------
        component ProcessX() -> () {
          X = prim std_mem_d1_ext(32, 4, 2);
          ...
        }

        ------------- Externalize Pass ---------------
        component ProcessX
        (go: 1, clk: 1, X0_read_data: 32, X0_done: 1) ->
        (done: 1, X0_addr0: 2, X0_write_data: 32, X0_write_en: 1, X0_clk: 1) {
           ...
        }

    '''
    program_string = ""
    for line in prog.splitlines(): program_string += f'{line}\n'

    with NamedTemporaryFile() as tf0, NamedTemporaryFile() as tf1, NamedTemporaryFile() as tf2:
        tf0.write(bytes(program_string, 'UTF-8'))
        tf0.seek(0)
        tf1.seek(0)
        tf2.seek(0)
        fuse_binary = os.environ['DAHLIA_EXEC'] if 'DAHLIA_EXEC' in os.environ else 'fuse'
        command = \
            f"""
            {fuse_binary} {tf0.name} --lower -b=futil -n={component_name} > {tf1.name} {NO_ERR} \
            && cd ../../ && cargo run -- {tf1.name} -p externalize > {tf2.name} {NO_ERR} 
            """
        subprocess.Popen(command, stdout=subprocess.PIPE, shell=True).communicate()
        component = tf2.read().decode()[len(IMPORT_STATEMENT):]  # Skip over importing the primitives library.
        return component


def tensor1d_op(declaration):
    op1, op2, res = declaration.inputs[0].primitive, declaration.inputs[1].primitive, declaration.output.primitive

    assert op1.type == PrimitiveType.Memory1D and op1.type == op2.type and op2.type == res.type
    assert op1.data[0] == op2.data[0] and op1.data[0] == res.data[0] and op1.data[1] == op2.data[1]
    assert op1.data[2] == op2.data[2] and op2.data[2] == res.data[2] and op2.data[1] == res.data[1]
    bitwidth, size, index_size = op1.data[0], op1.data[1], op1.data[2]
    return lower_dahlia_program(f"""
    decl {op1.name}: ubit<{bitwidth}>[{size}];
    decl {op2.name}: ubit<{bitwidth}>[{size}];
    decl {res.name}: ubit<{bitwidth}>[{size}];
    for (let i: ubit<{index_size}> = 0..{size}) {{
      {res.name}[i] := {op1.name}[i] {declaration.op} {op2.name}[i];
    }}""", declaration.component_name)


def tensor2d_op(declaration):
    op1, op2, res = declaration.inputs[0].primitive, declaration.inputs[1].primitive, declaration.output.primitive
    bitwidth, size0, size1, index_size0, index_size1 = op1.data[0], op1.data[1], op1.data[2], op1.data[3], op1.data[4]
    assert op1.type == PrimitiveType.Memory2D and op1.type == op2.type and op2.type == res.type
    assert bitwidth == op2.data[0] and op1.data[0] == res.data[0] and op2.data[4] == res.data[4]
    assert size0 == op2.data[1] and op2.data[1] == res.data[1] and size1 == op2.data[2] and op2.data[2] == res.data[2]
    assert index_size0 == op2.data[3] and op2.data[3] == res.data[3] and index_size1 == op2.data[4]

    return lower_dahlia_program(f"""
    decl {op1.name}: ubit<{bitwidth}>[{size0}][{size1}];
    decl {op2.name}: ubit<{bitwidth}>[{size0}][{size1}];
    decl {res.name}: ubit<{bitwidth}>[{size0}][{size1}];
    for (let i: ubit<{index_size0}> = 0..{size0}) {{
      for (let j: ubit<{index_size1}> = 0..{size1}) {{
        {res.name}[i][j] := {op1.name}[i][j] {declaration.op} {op2.name}[i][j];
      }}
    }}""", declaration.component_name)


def tensor3d_batch_flatten(declaration):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.batch_flatten"""
    op1, res = declaration.inputs[0].primitive, declaration.output.primitive
    bitwidth, op1_size0, op1_size1, op1_size2 = op1.data[0], op1.data[1], op1.data[2], op1.data[3]
    op1_index_size0, op1_index_size1, op1_index_size2 = op1.data[4], op1.data[5], op1.data[6]
    res_bitwidth, res_size0, res_size1 = res.data[0], res.data[1], res.data[2]
    res_index_size0, res_index_size1 = res.data[3], res.data[4]

    assert op1.type == PrimitiveType.Memory3D and res_size1 == op1_size1 * op1_size2 and res_size0 == op1_size0
    assert res.type == PrimitiveType.Memory2D and res_bitwidth == bitwidth
    return lower_dahlia_program(f"""
        decl {op1.name}: ubit<{bitwidth}>[{op1_size0}][{op1_size1}][{op1_size2}];
        decl {res.name}: ubit<{bitwidth}>[{res_size0}][{res_size1}];
        let l: ubit<{res_index_size1}> = 0;
        for (let i: ubit<{op1_index_size0}> = 0..{op1_size0}) {{
          for (let j: ubit<{op1_index_size1}> = 0..{op1_size1}) {{
            for (let k: ubit<{op1_index_size2}> = 0..{op1_size2}) {{
              {res.name}[i][l] := {op1.name}[i][j][k];
              l := l + 1;
            }}
          }}
        }}""", declaration.component_name)


def batch_matmul(declaration):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.batch_matmul"""
    assert False, "Unimplemented. nn.batch_matmul currently does not execute properly."
    op1, op2, res = declaration.inputs[0].primitive, declaration.inputs[1].primitive, declaration.output.primitive
    bitwidth, M1_size0, M1_size1, M1_size2 = op1.data[0], op1.data[1], op1.data[2], op1.data[3]
    M1_index_size0, M1_index_size1, M1_index_size2 = op1.data[4], op1.data[5], op1.data[6]
    M2_size0, M2_size1, M2_size2 = op2.data[1], op2.data[2], op2.data[3]
    M2_index_size0, M2_index_size1, M2_index_size2 = op2.data[4], op2.data[5], op2.data[6]
    assert op1.type == PrimitiveType.Memory3D and op1.type == op2.type and op2.type == res.type
    assert M2_size1 == M1_size2 and bitwidth == op2.data[0] and M1_size0 == M2_size0

    return lower_dahlia_program(f"""
    decl {op1.name}: ubit<{bitwidth}>[{M1_size0}][{M1_size1}][{M1_size2}];
    decl {op2.name}: ubit<{bitwidth}>[{M2_size0}][{M2_size1}][{M2_size2}];
    decl {res.name}: ubit<{bitwidth}>[{M1_size0}][{M1_size1}][{M2_size2}];
    for (let i: ubit<{M1_index_size0}> = 0..{M1_size0}) {{
      for (let j: ubit<{M1_index_size1}> = 0..{M1_size1}) {{
        for (let k: ubit<{M2_index_size2}> = 0..{M2_size2}) {{
          for (let l: ubit<{M1_index_size2}> = 0..{M1_size2}) {{
            let prod = {op1.name}[i][j][l] * {op2.name}[i][l][k];
          }} combine {{
            {res.name}[i][j][k] += prod;
          }}
        }}
      }}
    }}""", declaration.component_name)
