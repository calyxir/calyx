import subprocess

from tempfile import NamedTemporaryFile, TemporaryFile
from futil_ast import *

IMPORT_STATEMENT = """import "primitives/std.lib";\n"""
NO_ERR = "2>/dev/null"


def lower_dahlia_program(prog, component_name):
    '''
    Takes in a string that represents a Dahlia program, lowers it to FuTIL, and applies the `externalize` pass.
    This is just for experimental purposes, and needs to be replaced.
    More bluntly, this does the following:
    1. Copies dahlia program `prog` to a temporary file `temp.fuse`.
       $ echo `program_string` > temp.fuse

    2. Lowers `temp.fuse` to FuTIL with the name changed to `component_name`, and saves it in `lowered.futil`.
       $ ./fuse temp.fuse --lower -b=futil -n=component_name > lowered.futil

    3. Runs the 'externalize' pass on the `lowered.futil` file.
       $ cargo run -- lowered.futil -p externalize > temp.futil

    4. Copies the output from `lowered.futil`, except for the first line (we don't want another copy of the import).

    TODO(cgyurgyik): As you'll see below, this only works on my local machine.
                     I've explicitly removed errors with `2>/dev/null` so they aren't inserted
                     to the file as well. However, this makes debugging difficult as well.
    '''
    program_string = ""
    for line in prog.splitlines(): program_string += f'{line}\n'

    with NamedTemporaryFile() as tf0, NamedTemporaryFile() as tf1, NamedTemporaryFile() as tf2:
        tf0.write(bytes(program_string, 'UTF-8'))
        tf0.seek(0)
        tf1.seek(0)
        tf2.seek(0)
        command = \
            f"""
            fuse {tf0.name} --lower -b=futil -n={component_name} > {tf1.name} \
            {NO_ERR} && cd ../../ && cargo run -- {tf1.name} -p externalize > {tf2.name} {NO_ERR} 
            """
        subprocess.Popen(command, stdout=subprocess.PIPE, shell=True).communicate()
        component = open(tf2.name, 'r').read()[len(IMPORT_STATEMENT):]  # Skip over importing the primitives library.
        return component


def tensor1d_op(declaration):
    op1 = declaration.inputs[0].primitive
    op2 = declaration.inputs[1].primitive
    res = declaration.output.primitive

    assert op1.type == PrimitiveType.Memory1D and op1.type == op2.type and op2.type == res.type
    assert op1.data[0] == op2.data[0] and op1.data[0] == res.data[0]
    assert op1.data[1] == op2.data[1] and op2.data[1] == res.data[1]
    assert op1.data[2] == op2.data[2] and op2.data[2] == res.data[2]
    bitwidth = op1.data[0]
    size = op1.data[1]
    index_size = op1.data[2]
    return lower_dahlia_program(f"""
    decl {op1.name}: ubit<{bitwidth}>[{size}];
    decl {op2.name}: ubit<{bitwidth}>[{size}];
    decl {res.name}: ubit<{bitwidth}>[{size}];
    for (let i: ubit<{index_size}> = 0..{size}) {{
      {res.name}[i] := {op1.name}[i] {declaration.op} {op2.name}[i];
    }}""", declaration.component_name)


def tensor2d_op(declaration):
    op1 = declaration.inputs[0].primitive
    op2 = declaration.inputs[1].primitive
    res = declaration.output.primitive

    assert op1.type == PrimitiveType.Memory2D and op1.type == op2.type and op2.type == res.type
    assert op1.data[0] == op2.data[0] and op1.data[0] == res.data[0]
    assert op1.data[1] == op2.data[1] and op2.data[1] == res.data[1]
    assert op1.data[2] == op2.data[2] and op2.data[2] == res.data[2]
    assert op1.data[3] == op2.data[3] and op2.data[3] == res.data[3]
    assert op1.data[4] == op2.data[4] and op2.data[4] == res.data[4]

    bitwidth = op1.data[0]
    size0 = op1.data[1]
    size1 = op1.data[2]
    index_size0 = op1.data[3]
    index_size1 = op1.data[4]
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
    op1 = declaration.inputs[0].primitive
    res = declaration.output.primitive

    bitwidth = op1.data[0]
    op1_size0 = op1.data[1]
    op1_size1 = op1.data[2]
    op1_size2 = op1.data[3]
    op1_index_size0 = op1.data[4]
    op1_index_size1 = op1.data[5]
    op1_index_size2 = op1.data[6]
    res_bitwidth = res.data[0]
    res_size0 = res.data[1]
    res_size1 = res.data[2]
    res_index_size0 = res.data[3]
    res_index_size1 = res.data[4]

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
