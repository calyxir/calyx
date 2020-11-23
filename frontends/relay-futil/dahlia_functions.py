import subprocess
import os

from tempfile import NamedTemporaryFile, TemporaryFile
from futil_ast import *

IMPORT_STATEMENT = """import "primitives/std.lib";\n"""
NO_ERR = "2>/dev/null"
CHARACTER_I = chr(ord('i'))


def lower_dahlia_program(prog, component_name):
    """
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
    """
    program_string = '\n'.join(prog.splitlines())
    with NamedTemporaryFile() as tf0, NamedTemporaryFile() as tf1, NamedTemporaryFile() as tf2:
        tf0.write(bytes(program_string, 'UTF-8'))
        tf0.seek(0), tf1.seek(0), tf2.seek(0)
        fuse_binary = os.environ['DAHLIA_EXEC'] if 'DAHLIA_EXEC' in os.environ else 'fuse'
        command = f"""
                {fuse_binary} {tf0.name} --lower -b=futil -n={component_name} > {tf1.name} {NO_ERR} \
                 && cargo run -- {tf1.name} -l ../../ -p externalize > {tf2.name} {NO_ERR}"""
        subprocess.Popen(command, stdout=subprocess.PIPE, shell=True).communicate()
        component = tf2.read().decode()[len(IMPORT_STATEMENT):]  # Skip over importing the primitives library.
        return component


def next_character(ch, dir=1):
    """
    Returns the next character after 'ch'.
    If dir is positive, then will return 'ch' + 1. Otherwise, it will return 'ch' - 1.
    """
    return chr(ord(ch) + dir) if dir > 0 else chr(ord(ch) - 1)


def broadcast(declaration):
    """
    https://numpy.org/doc/stable/user/basics.broadcasting.html
    Implements array broadcasting:
    Two dimensions are compatible when either (1) they're equal, or (2) one of them is 1.
    It is not required that both operands have the same number of dimensions either.
    - When lowering from Relay IR, we are guaranteed the arrays are compatible for broadcasting.
    - Variable names for indexing through the array begin with `i`, and continue alphabetically.

    Example:
         first operand:  64 x  1 x 32
        second operand:       16 x  1
                result:  64 x 16 x 32
        ->
        for (i = 0...64) {
          for (j = 0..16) {
            for (k = 0..32) {
              result[i][j][k] := op1[i][0][k] + op2[j][0];
              ...
    """
    operand1, operand2 = declaration.inputs[0].primitive, declaration.inputs[1].primitive
    res = declaration.output.primitive
    op1 = operand1 if operand1.type >= operand2.type else operand2
    op2 = operand2 if op1 == operand1 else operand1

    op1_offset, op2_offset = op1.type, op2.type
    op1_sizes, op2_sizes, res_sizes = [], [], []
    for i in reversed(range(1, op1_offset + 1)): op1_sizes.append(op1.data[i])
    for i in reversed(range(1, op2_offset + 1)): op2_sizes.append(op2.data[i])
    for i in range(0, len(op1_sizes)):
        size = op1_sizes[i]
        res_sizes.append(max(size, op2_sizes[i]) if i < len(op2_sizes) else size)

    op1_indices, op2_indices, res_indices = [], [], []
    # Get the character associated with 'i' + N, where N == number of dimensions in `op1`.
    variable_name = chr(ord(CHARACTER_I) + op1_offset - 1)
    for i in range(0, len(op1_sizes)):
        current_dimension, index_zero = f'[{variable_name}]', '[0]'
        res_indices.append(current_dimension)
        if len(op2_sizes) <= i:
            op1_indices.append(current_dimension)
            continue
        elif op1_sizes[i] == op2_sizes[i]:
            op1_indices.append(current_dimension)
            op2_indices.append(current_dimension)
        elif op1_sizes[i] > op2_sizes[i]:
            op1_indices.append(current_dimension)
            op2_indices.append(index_zero)
        else:  # op2_sizes[i] < op1_sizes[i]
            op1_indices.append(index_zero)
            op2_indices.append(current_dimension)
        variable_name = next_character(variable_name, -1)

    # Resulting index in the nested for loop, e.g. for op1[i][j][0][k], this is `[i][j][0][k]`.
    op1_index, op2_index = ''.join(reversed(op1_indices)), ''.join(reversed(op2_indices))
    res_index = ''.join(reversed(res_indices))

    # Declarations for op1, op2, res.
    op1_decl = f'decl {op1.name}: {op1.data_type}<{op1.data[0]}>'
    op2_decl = f'decl {op2.name}: {op2.data_type}<{op2.data[0]}>'
    res_decl = f'decl {res.name}: {res.data_type}<{res.data[0]}>'
    for i in reversed(range(0, len(op1_sizes))): op1_decl += f'[{op1_sizes[i]}]'
    for i in reversed(range(0, len(op2_sizes))): op2_decl += f'[{op2_sizes[i]}]'
    for i in reversed(range(0, len(res_sizes))): res_decl += f'[{res_sizes[i]}]'

    # For loop(s).
    variable_name = CHARACTER_I
    loop_body = []
    for i in range(1, len(op1_sizes) + 1):
        size, index_size = res.data[i], res.data[i + op1_offset]
        if (i + op2_offset < len(op2_sizes)):
            op2_size, op2_index_size = op2.data[i], op2.data[i + op2_offset]
            size, index_size = max(size, op2_size), max(size, op2_index_size)
        loop_body.append(f'for (let {variable_name}: ubit<{index_size}> = 0..{size}) {{')
        variable_name = next_character(variable_name)
    loop_body.append(f'{res.name}{res_index} := {op1.name}{op1_index} {declaration.op} {op2.name}{op2_index};')

    for i in range(1, len(op1_sizes) + 1): loop_body.append('}')
    program = f"""
    {op1_decl};
    {op2_decl};
    {res_decl};
    {' '.join(loop_body)}
    """
    return lower_dahlia_program(program, declaration.component_name)


def batch_flatten(declaration):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.batch_flatten"""
    op1, res = declaration.inputs[0].primitive, declaration.output.primitive
    bitwidth, res_bitwidth, res_size0, res_size1 = op1.data[0], res.data[0], res.data[1], res.data[2]
    res_index_size0, res_index_size1 = res.data[3], res.data[4]

    if op1.type == PrimitiveType.Memory3D:
        op1_size0, op1_size1, op1_size2 = op1.data[1], op1.data[2], op1.data[3]
        op1_index_size0, op1_index_size1, op1_index_size2 = op1.data[4], op1.data[5], op1.data[6]
        program = f"""
            decl {op1.name}: {op1.data_type}<{bitwidth}>[{op1_size0}][{op1_size1}][{op1_size2}];
            decl {res.name}: {res.data_type}<{bitwidth}>[{res_size0}][{res_size1}];
            let l: ubit<{res_index_size1}> = 0;
            for (let i: ubit<{op1_index_size0}> = 0..{op1_size0}) {{
              for (let j: ubit<{op1_index_size1}> = 0..{op1_size1}) {{
                for (let k: ubit<{op1_index_size2}> = 0..{op1_size2}) {{
                  {res.name}[i][l] := {op1.name}[i][j][k];
                  l := l + 1;
                }}
              }}
            }}"""
        return lower_dahlia_program(program, declaration.component_name)
    if op1.type == PrimitiveType.Memory4D:
        op1_size0, op1_size1, op1_size2, op1_size3 = op1.data[1], op1.data[2], op1.data[3], op1.data[4]
        op1_index_size0, op1_index_size1 = op1.data[5], op1.data[6]
        op1_index_size2, op1_index_size3 = op1.data[7], op1.data[8]
        program = f"""
            decl {op1.name}: {op1.data_type}<{bitwidth}>[{op1_size0}][{op1_size1}][{op1_size2}][{op1_size3}];
            decl {res.name}: {res.data_type}<{bitwidth}>[{res_size0}][{res_size1}];
            let l: ubit<{res_index_size1}> = 0;
            for (let i: ubit<{op1_index_size0}> = 0..{op1_size0}) {{
              for (let j: ubit<{op1_index_size1}> = 0..{op1_size1}) {{
                for (let k: ubit<{op1_index_size2}> = 0..{op1_size2}) {{
                  for (let l: ubit<{op1_index_size3}> = 0..{op1_size3}) {{
                    {res.name}[i][l] := {op1.name}[i][j][k][l];
                    l := l + 1;
                  }}
                }}
              }}
            }}"""
        return lower_dahlia_program(program, declaration.component_name)


def bias_add(declaration):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.bias_add"""
    axis = declaration.attributes.get_int("axis")
    data, bias, res = declaration.inputs[0].primitive, declaration.inputs[1].primitive, declaration.output.primitive
    bitwidth = data.data[0]
    if data.type == PrimitiveType.Memory2D:
        size0, size1, index_size0, index_size1 = data.data[1], data.data[2], data.data[3], data.data[4]
        bias_size, bias_index_size = bias.data[1], bias.data[2]
        program = f"""
        decl {data.name}: {data.data_type}<{bitwidth}>[{size0}][{size1}];
        decl {bias.name}: {bias.data_type}<{bitwidth}>[{bias_size}];
        decl {res.name}: {res.data_type}<{bitwidth}>[{size0}][{size1}];"""
        if axis == 1:
            program += f"""
            for (let i: ubit<{index_size0}> = 0..{size0}) {{
              for (let j: ubit<{index_size1}> = 0..{size1}) {{
                {res.name}[i][j] := {data.name}[i][j] + {bias.name}[j];
              }}
            }}"""
        elif axis == 0:
            program += f"""
            for (let j: ubit<{index_size1}> = 0..{size1}) {{
              for (let i: ubit<{index_size0}> = 0..{size0}) {{
                {res.name}[i][j] := {data.name}[i][j] + {bias.name}[i];
              }}
            }}"""
    elif data.type == PrimitiveType.Memory4D:
        bitwidth, size0, size1, size2, size3 = data.data[0], data.data[1], data.data[2], data.data[3], data.data[4]
        index_size0, index_size1, index_size2, index_size3 = data.data[5], data.data[6], data.data[7], data.data[8]
        bias_size, bias_index_size = bias.data[1], bias.data[2]
        program = f"""
        decl {data.name}: {data.data_type}<{bitwidth}>[{size0}][{size1}][{size2}][{size3}];
        decl {bias.name}: {bias.data_type}<{bitwidth}>[{bias_size}];
        decl {res.name}: {res.data_type}<{bitwidth}>[{size0}][{size1}][{size2}][{size3}];"""
        if axis == 1:
            program += f"""
            for (let i: ubit<{index_size0}> = 0..{size0}) {{
              for (let j: ubit<{index_size1}> = 0..{size1}) {{
                for (let k: ubit<{index_size2}> = 0..{size2}) {{
                  for (let l: ubit<{index_size3}> = 0..{size3}) {{
                    {res.name}[i][j][k][l] := {data.name}[i][j][k][l] + {bias.name}[j];
                  }}
                }}
              }}
            }}"""

    return lower_dahlia_program(program, declaration.component_name)


# TODO(cgyurgyik):
#  1. This won't work for fixed point currently, since Dahlia
#     will not take fixed point operands for the `>` operator.
#  2. Without signed bit array support, this is also meaningless.
def relu(declaration):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.relu"""
    op1, res = declaration.inputs[0].primitive, declaration.output.primitive
    assert res.data_type == 'ubit', f'{res.data_type} is not currently supported for ReLU.'

    if op1.type == PrimitiveType.Memory2D:
        bitwidth, op1_size0, op1_size1 = op1.data[0], op1.data[1], op1.data[2]
        op1_index_size0, op1_index_size1 = op1.data[3], op1.data[4]
        res_bitwidth, res_size0, res_size1 = res.data[0], res.data[1], res.data[2]
        res_index_size0, res_index_size1 = res.data[3], res.data[4]
        program = f"""
        decl {op1.name}: {op1.data_type}<{bitwidth}>[{op1_size0}][{op1_size1}];
        decl {res.name}: {res.data_type}<{bitwidth}>[{res_size0}][{res_size1}];
        let zero: {op1.data_type}<{bitwidth}> = 0;
        for (let i: ubit<{op1_index_size0}> = 0..{op1_size0}) {{
          for (let j: ubit<{op1_index_size1}> = 0..{op1_size1}) {{
            if ({op1.name}[i][j] > zero) {{
              {res.name}[i][j] := {op1.name}[i][j];
            }} else {{
              {res.name}[i][j] := 0;
            }}
          }}
        }}
        """
        return lower_dahlia_program(program, declaration.component_name)

    elif op1.type == PrimitiveType.Memory4D:
        bitwidth, op1_size0, op1_size1 = op1.data[0], op1.data[1], op1.data[2]
        op1_size2, op1_size3, op1_index_size0, = op1.data[3], op1.data[4], op1.data[5]
        op1_index_size1, op1_index_size2, op1_index_size3 = op1.data[6], op1.data[7], op1.data[8]
        res_bitwidth, res_size0, res_size1 = res.data[0], res.data[1], res.data[2]
        res_size2, res_size3, res_index_size0, res_index_size1 = res.data[3], res.data[4], res.data[5], res.data[6]
        res_index_size2, res_index_size3 = res.data[7], res.data[8]

        program = f"""
                decl {op1.name}: {op1.data_type}<{bitwidth}>[{op1_size0}][{op1_size1}][{op1_size2}][{op1_size3}];
                decl {res.name}: {res.data_type}<{bitwidth}>[{res_size0}][{res_size1}][{op1_size2}][{op1_size3}];
                let zero: {op1.data_type}<{bitwidth}> = 0;
                for (let i: ubit<{op1_index_size0}> = 0..{op1_size0}) {{
                  for (let j: ubit<{op1_index_size1}> = 0..{op1_size1}) {{
                    for (let k: ubit<{op1_index_size2}> = 0..{op1_size2}) {{
                      for (let l: ubit<{op1_index_size3}> = 0..{op1_size3}) {{
                        if ({op1.name}[i][j][k][l] > zero) {{
                          {res.name}[i][j][k][l] := {op1.name}[i][j][k][l];
                        }} else {{
                          {res.name}[i][j][k][l] := 0;
                        }}
                      }} 
                    }}
                  }}
                }}
                """
        return lower_dahlia_program(program, declaration.component_name)


# TODO(cgyurgyik): Similar to ReLU, this requires signed operands.
def negative(declaration):
    """https://tvm.apache.org/docs/api/python/relay/index.html#tvm.relay.negative"""
    op1, res = declaration.inputs[0].primitive, declaration.output.primitive
    bitwidth, size, index_size = op1.data[0], op1.data[1], op1.data[2]
    program = f"""
        decl {op1.name}: {op1.data_type}<{bitwidth}>[{size}];
        decl {res.name}: {res.data_type}<{bitwidth}>[{size}];
        for (let i: ubit<{index_size}> = 0..{size}) {{
          {res.name}[i] := -{op1.name}[i];
        }}
    """
    return lower_dahlia_program(program, declaration.component_name)


def expand_dims(declaration):
    """https://tvm.apache.org/docs/api/python/relay/index.html#tvm.relay.expand_dims"""
    axis, num_newaxis = declaration.attributes.get_int("axis"), declaration.attributes.get_int("num_newaxis")
    data, res = declaration.inputs[0].primitive, declaration.output.primitive
    bitwidth, size, index_size = data.data[0], data.data[1], data.data[2]
    size0, size1, size2 = res.data[1], res.data[2], res.data[3]
    index_size0, index_size1, index_size2 = res.data[4], res.data[5], res.data[6]
    if axis == 1 and num_newaxis == 2:
        program = f"""
        decl {data.name}: {data.data_type}<{bitwidth}>[{size}];
        decl {res.name}: {res.data_type}<{bitwidth}>[{size0}][{size1}][{size2}];
        for (let i: ubit<{index_size}> = 0..{size}) {{
          {res.name}[i][0][0] := {data.name}[i];
        }}
        """
    return lower_dahlia_program(program, declaration.component_name)


def batch_matmul(declaration):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.batch_matmul"""
    op1, op2, res = declaration.inputs[0].primitive, declaration.inputs[1].primitive, declaration.output.primitive
    bitwidth, M1_size0, M1_size1, M1_size2 = op1.data[0], op1.data[1], op1.data[2], op1.data[3]
    M1_index_size0, M1_index_size1, M1_index_size2 = op1.data[4], op1.data[5], op1.data[6]
    M2_size0, M2_size1, M2_size2 = op2.data[1], op2.data[2], op2.data[3]
    M2_index_size0, M2_index_size1, M2_index_size2 = op2.data[4], op2.data[5], op2.data[6]
    # 1. Get transpose of second operand.
    # 2. Create temporary value `t`. Then, t = op1 * transpose(op2).
    # 3. Copy temporary value to return value.*
    #    * This third step may not be necessary, but trying to conduct the matrix multiply
    #      directly with the return value declared resulted in incorrect outputs.
    program = f"""
    decl {op1.name}: {op1.data_type}<{bitwidth}>[{M1_size0}][{M1_size1}][{M1_size2}];
    decl {op2.name}: {op2.data_type}<{bitwidth}>[{M2_size0}][{M2_size1}][{M2_size2}];
    decl {res.name}: {res.data_type}<{bitwidth}>[{M1_size0}][{M1_size1}][{M2_size1}];
    let transpose_{op2.name}: {op2.data_type}<{bitwidth}>[{M2_size0}][{M2_size2}][{M2_size1}];
    let temporary_{res.name}: {res.data_type}<{bitwidth}>[{M1_size0}][{M1_size1}][{M2_size1}];
    for (let batch: ubit<{M1_index_size0}> = 0..{M1_size0}) {{
      for (let i: ubit<{M2_index_size1}> = 0..{M2_size1}) {{
        for (let j: ubit<{M2_index_size2}> = 0..{M2_size2}) {{
          transpose_{op2.name}[batch][j][i] := {op2.name}[batch][i][j];
        }}
      }}
    }} 

    for (let batch: ubit<{M1_index_size0}> = 0..{M1_size0}) {{
      for (let i: ubit<{M1_index_size1}> = 0..{M1_size1}) {{
        for (let j: ubit<{M2_index_size1}> = 0..{M2_size1}) {{
          for (let k: ubit<{M2_index_size2}> = 0..{M2_size2}) {{
            let product = {op1.name}[batch][i][k] * transpose_{op2.name}[batch][k][j];
          }} combine {{
            temporary_{res.name}[batch][i][j] += product;
          }}
        }}
      }}
    }}

    for (let batch: ubit<{M1_index_size0}> = 0..{M1_size0}) {{
      for (let i: ubit<{M1_index_size1}> = 0..{M1_size1}) {{
        for (let j: ubit<{M2_index_size1}> = 0..{M2_size1}) {{
          {res.name}[batch][i][j] := temporary_{res.name}[batch][i][j];
        }}
      }}
    }} 
    """
    return lower_dahlia_program(program, declaration.component_name)


# TODO(cgyurgyik): Similar to batch_matmul, this requires a temporary memory to store the output
# of the matrix multiply. Otherwise, the values aren't computed properly. Look deeper into this.
def dense(declaration):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.batch_matmul"""
    # TODO(cgyurgyik): Add support for `units`.
    units = declaration.attributes.get_int("units")
    op1, op2, res = declaration.inputs[0].primitive, declaration.inputs[1].primitive, declaration.output.primitive
    bitwidth, M1_size0, M1_size1 = op1.data[0], op1.data[1], op1.data[2]
    M1_index_size0, M1_index_size1 = op1.data[3], op1.data[4]
    M2_size0, M2_size1, M2_index_size0, M2_index_size1 = op2.data[1], op2.data[2], op2.data[3], op2.data[4]
    program = f"""
    decl {op1.name}: {op1.data_type}<{bitwidth}>[{M1_size0}][{M1_size1}];
    decl {op2.name}: {op2.data_type}<{bitwidth}>[{M2_size0}][{M2_size1}];
    decl {res.name}: {res.data_type}<{bitwidth}>[{M1_size0}][{M2_size0}];
    let transpose_{op2.name}: {op2.data_type}<{bitwidth}>[{M2_size1}][{M2_size0}];
    let temporary_{res.name}: {res.data_type}<{bitwidth}>[{M1_size0}][{M2_size0}];
    for (let i: ubit<{M2_index_size0}> = 0..{M2_size0}) {{
      for (let j: ubit<{M2_index_size1}> = 0..{M2_size1}) {{
        transpose_{op2.name}[j][i] := {op2.name}[i][j];
      }}
    }} 

    for (let i: ubit<{M1_index_size0}> = 0..{M1_size0}) {{
      for (let j: ubit<{M2_index_size0}> = 0..{M2_size0}) {{
        for (let k: ubit<{M1_index_size1}> = 0..{M1_size1}) {{
          let product = {op1.name}[i][k] * transpose_{op2.name}[k][j];
        }} combine {{
          temporary_{res.name}[i][j] += product;
        }}
      }}
    }}

    for (let i: ubit<{M1_index_size0}> = 0..{M1_size0}) {{
      for (let j: ubit<{M2_index_size0}> = 0..{M2_size0}) {{
        {res.name}[i][j] := temporary_{res.name}[i][j];
      }}
    }}
    """
    return lower_dahlia_program(program, declaration.component_name)
