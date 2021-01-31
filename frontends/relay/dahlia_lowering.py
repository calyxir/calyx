import subprocess
import os

from tempfile import NamedTemporaryFile, TemporaryFile
from futil_ast import *

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


def PPDahliaMemoryDeclarations(relay_function):
    """
    Pretty print for Dahlia memory declarations, e.g.
    `decl X: ubit<32> [1][10];`
    """
    cell_list = relay_function.inputs
    cell_list.append(relay_function.output)

    declarations = []
    for cell in cell_list:
        declaration = cell.primitive
        declaration_str = f'decl {declaration.name}: {declaration.data_type}<{declaration.data[0]}>'
        for i in range(0, declaration.type): declaration_str += f'[{declaration.data[i + 1]}]'
        declarations.append(declaration_str + ";")
    return '\n'.join(declarations)


def PPDahliaLoop(relay_function, body, num_dimensions, data=None):
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
    output = relay_function.output.primitive if data == None else data
    for i in range(0, num_dimensions):
        size, index_size = output.data[i + 1], output.data[i + num_dimensions + 1]
        program.append(f'{SPACING}for (let {variable_name}: ubit<{index_size}> = 0..{size}) {{')
        variable_name = next_character(variable_name)
        SPACING += '  '
    program.append(f'{SPACING}{body}')

    for i in range(0, num_dimensions):
        SPACING = SPACING[:-2]
        program.append(SPACING + '}')
    return '\n'.join(program)


def LowerDahliaProgramToFuTIL(relay_function, dahlia_body, dahlia_imports=None):
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

        ------------- Externalize Pass ---------------
        component ProcessX
        (go: 1, clk: 1, X0_read_data: 32, X0_done: 1) ->
        (done: 1, X0_addr0: 2, X0_write_data: 32, X0_write_en: 1, X0_clk: 1) {
           ...
        }
    """
    if dahlia_imports == None: dahlia_imports = ''
    program_string = '\n'.join((dahlia_imports, PPDahliaMemoryDeclarations(relay_function), dahlia_body))

    with NamedTemporaryFile() as tf0, NamedTemporaryFile() as tf1, NamedTemporaryFile() as tf2:
        tf0.write(bytes(program_string, 'UTF-8'))
        tf0.seek(0), tf1.seek(0), tf2.seek(0)
        fuse_binary = os.environ['DAHLIA_EXEC'] if 'DAHLIA_EXEC' in os.environ else 'fuse'
        command = f"""
                {fuse_binary} {tf0.name} --lower -b=futil -n={relay_function.component_name} > {tf1.name} {NO_ERR} \
                 && fud e --from futil {tf1.name} --to futil-externalize > {tf2.name} {NO_ERR}"""
        subprocess.Popen(command, stdout=subprocess.PIPE, shell=True).communicate()
        component = tf2.read().decode()[len(IMPORT_STATEMENT):]  # Skip over importing the primitives library.
        return component


####################################################################################################
################## Dahlia Implementations for Relay Function Calls #################################
####################################################################################################

def broadcast(function: RelayFunctionCall):
    """
    https://numpy.org/doc/stable/user/basics.broadcasting.html
    Implements array broadcasting:
    Two dimensions are compatible when either (1) they're equal, or (2) one of them is `1`.
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
              result[i][j][k] := op1[i][0][k] op op2[j][0];
              ...
    """
    op1, op2, res = function.inputs[0].primitive, function.inputs[1].primitive, function.output.primitive
    op1_dims, op2_dims, res_dims = op1.type, op2.type, res.type
    op1_sizes, op2_sizes, res_sizes = [], [], []
    # Get memory sizes in reversed order.
    for i in reversed(range(0, op1_dims)): op1_sizes.append(op1.data[i + 1])
    for i in reversed(range(0, op2_dims)): op2_sizes.append(op2.data[i + 1])
    for i in reversed(range(0, res_dims)): res_sizes.append(res.data[i + 1])

    # Gets the last variable name since we will compare sizes in the reverse direction.
    variable_name = chr(ord(CHARACTER_I) + res_dims - 1)
    # Determine the value at the indices in reverse order.
    # For each dimension, this will either be `[x]` for index_variable `x`, or `[0]`
    # depending on the relationship between the dimensions sizes.
    op1_indices, op2_indices, res_indices = [], [], []
    for i in range(0, len(res_sizes)):
        current_dimension, index_zero = f'[{variable_name}]', '[0]'
        res_indices.append(current_dimension)
        if op1_dims > op2_dims and len(op2_sizes) <= i:
            op1_indices.append(current_dimension)
        elif op2_dims > op1_dims and len(op1_sizes) <= i:
            op2_indices.append(current_dimension)
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

    # Resulting index in the nested for loop, e.g. for `op1[i][j][0][k]`, this is `[i][j][0][k]`.
    op1_index = ''.join(reversed(op1_indices))
    op2_index = ''.join(reversed(op2_indices))
    res_index = ''.join(reversed(res_indices))
    loop_body = f'{res.name}{res_index} := {op1.name}{op1_index} {function.op} {op2.name}{op2_index};'

    return LowerDahliaProgramToFuTIL(function, PPDahliaLoop(function, loop_body, num_dimensions=res_dims))


def batch_flatten(function):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.batch_flatten"""
    data, res = function.inputs[0].primitive, function.output.primitive
    bitwidth, num_dimensions = res.data[0], data.type
    res_index_size1 = res.data[4]

    variable_name = CHARACTER_I
    data_indices, res_indices = "", f'[{variable_name}]'
    for i in range(0, num_dimensions):
        # Determine loop body indices based on `axis` provided.
        size, index_size = data.data[i + 1], data.data[i + num_dimensions + 1]
        index = f'[{variable_name}]'
        data_indices += index
        variable_name = next_character(variable_name)
    res_indices += f'[{variable_name}]'

    let_flattened = f'let {variable_name}: ubit<{res_index_size1}> = 0;'
    body = f"{res.name}{res_indices} := {data.name}{data_indices}; {variable_name} := {variable_name} + 1;"
    program_body = '\n'.join((let_flattened, PPDahliaLoop(function, body, num_dimensions, data)))
    return LowerDahliaProgramToFuTIL(function, program_body)


def bias_add(function):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.bias_add"""
    data, bias, res = function.inputs[0].primitive, function.inputs[1].primitive, function.output.primitive
    bitwidth, num_dimensions = data.data[0], data.type

    axis_attribute = function.attributes.get_int("axis")
    axis = num_dimensions - 1 if axis_attribute == -1 else axis_attribute

    variable_name = CHARACTER_I
    data_indices = ""
    for i in range(0, num_dimensions):
        # Determine loop body indices based on `axis` provided.
        size, index_size = data.data[i + 1], data.data[i + num_dimensions + 1]
        index = f'[{variable_name}]'
        if axis == i: bias_index = index
        data_indices += index
        variable_name = next_character(variable_name)

    body = f"{res.name}{data_indices} := {data.name}{data_indices} + {bias.name}{bias_index};"
    return LowerDahliaProgramToFuTIL(function, PPDahliaLoop(function, body, num_dimensions))


# TODO(cgyurgyik):
#  1. This won't work for fixed point currently, since Dahlia
#     will not take fixed point operands for the `>` operator.
#  2. Without signed bit array support, this is also meaningless.
def relu(function):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.relu"""
    data, res = function.inputs[0].primitive, function.output.primitive
    bitwidth, num_dimensions, data_type = data.data[0], data.type, data.data_type

    zero = '0.0' if data_type == 'ufix' or data_type == 'fix' else '0'
    let_zero = f'let zero: {data_type}<{bitwidth}> = {zero};'

    indices = ""
    variable_name = CHARACTER_I
    for i in range(0, num_dimensions):
        # Determine loop body indices.
        indices += f'[{variable_name}]'
        variable_name = next_character(variable_name)

    body = f"""if ({data.name}{indices} > zero) {{ {res.name}{indices} := {data.name}{indices}; }} 
               else {{ {res.name}{indices} := zero; }}"""
    program_body = '\n'.join((let_zero, PPDahliaLoop(function, body, num_dimensions)))
    return LowerDahliaProgramToFuTIL(function, program_body)


# TODO(cgyurgyik): Similar to ReLU, this requires signed operands.
def negative(function):
    """https://tvm.apache.org/docs/api/python/relay/index.html#tvm.relay.negative"""
    op, res = function.inputs[0].primitive, function.output.primitive
    bitwidth, num_dimensions, data_type = op.data[0], op.type, op.data_type

    indices = ""
    variable_name = CHARACTER_I
    for i in range(0, num_dimensions):
        # Determine loop body indices.
        indices += f'[{variable_name}]'
        variable_name = next_character(variable_name)

    zero = '0.0' if data_type == 'ufix' or data_type == 'fix' else '0'
    program_body = PPDahliaLoop(function, f"""{res.name}{indices} := {zero} - {op.name}{indices};""", num_dimensions)
    return LowerDahliaProgramToFuTIL(function, program_body)


def sqrt(function):
    """https://tvm.apache.org/docs/api/python/relay/index.html#tvm.relay.sqrt"""
    op, res = function.inputs[0].primitive, function.output.primitive
    bitwidth, num_dimensions, data_type = op.data[0], op.type, op.data_type
    include_sqrt = f"""import "fxp_sqrt.h" {{ def sqrt(value: {data_type}<{bitwidth}>): {data_type}<{bitwidth}>; }}"""

    indices = ""
    variable_name = CHARACTER_I
    for i in range(0, num_dimensions):
        # Determine loop body indices.
        indices += f'[{variable_name}]'
        variable_name = next_character(variable_name)

    program_body = PPDahliaLoop(function, f"""{res.name}{indices} := sqrt({op.name}{indices});""", num_dimensions)
    return LowerDahliaProgramToFuTIL(function, program_body, include_sqrt)


def expand_dims(function):
    """https://tvm.apache.org/docs/api/python/relay/index.html#tvm.relay.expand_dims"""
    axis, num_newaxis = function.attributes.get_int("axis"), function.attributes.get_int("num_newaxis")
    data, res = function.inputs[0].primitive, function.output.primitive
    bitwidth, num_dimensions = data.data[0], data.type

    res_indices, data_indices = "", ""
    variable_name = CHARACTER_I
    for i in range(0, num_dimensions):
        # Determine loop body indices.
        index = f'[{variable_name}]'
        res_indices += index
        data_indices += index
        if axis == i + 1:
            for _ in range(0, num_newaxis): res_indices += '[0]'
        variable_name = next_character(variable_name)

    program_body = PPDahliaLoop(function, f'{res.name}{res_indices} := {data.name}{data_indices}', num_dimensions, data)
    return LowerDahliaProgramToFuTIL(function, program_body)


def batch_matmul(function):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.batch_matmul"""
    op1, op2, res = function.inputs[0].primitive, function.inputs[1].primitive, function.output.primitive
    bitwidth, M1_size0, M1_size1, M1_size2 = op1.data[0], op1.data[1], op1.data[2], op1.data[3]
    M1_index_size0, M1_index_size1, M1_index_size2 = op1.data[4], op1.data[5], op1.data[6]
    M2_size0, M2_size1, M2_size2 = op2.data[1], op2.data[2], op2.data[3]
    M2_index_size0, M2_index_size1, M2_index_size2 = op2.data[4], op2.data[5], op2.data[6]
    # 1. Get transpose of second operand.
    # 2. Create temporary value `t`. Then, t = op1 * transpose(op2).
    # 3. Copy temporary value to return value.*
    #    * This third step may not be necessary, but trying to conduct the matrix multiply
    #      directly with the return value declared resulted in incorrect outputs.
    program_body = f"""
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
    return LowerDahliaProgramToFuTIL(function, program_body)


# TODO(cgyurgyik): Similar to batch_matmul, this requires a temporary memory to store the output
# of the matrix multiply. Otherwise, the values aren't computed properly. Look deeper into this.
def dense(function):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.dense"""
    op1, op2, res = function.inputs[0].primitive, function.inputs[1].primitive, function.output.primitive
    bitwidth, M1_size0, M1_size1 = op1.data[0], op1.data[1], op1.data[2]
    M1_index_size0, M1_index_size1 = op1.data[3], op1.data[4]
    M2_size0, M2_size1, M2_index_size0, M2_index_size1 = op2.data[1], op2.data[2], op2.data[3], op2.data[4]
    program = f"""
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
    return LowerDahliaProgramToFuTIL(function, program)


# TODO(cgyurgyik): Currently, only supports a small subset (namely those used in our VGG net and MLP net examples).
def softmax(function):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.softmax"""
    assert False, "Unimplemented."
    op, res = function.inputs[0].primitive, function.output.primitive
    axis = function.attributes.get_int("axis")
    data_type = op.data_type
    assert op.type == PrimitiveType.Memory2D, f'nn.softmax with pritmive type Memory{op.type}D is not supported.'
    assert axis == -1 or axis == 1, f'nn.softmax with axis = {axis} is not supported.'
    bitwidth, size0, size1, index_size0, index_size1 = op.data[0], op.data[1], op.data[2], op.data[3], op.data[4]

    import_exp = f"""import "std_exp.h" {{ def exp(x: {data_type}<{bitwidth}>): {data_type}<{bitwidth}>; }}"""
    zero = '0.0' if data_type == 'ufix' or data_type == 'fix' else '0'
    program_body = f"""
    for (let i: ubit<{index_size0}> = 0..{size0}) {{
      let {op.name}_expsum: {data_type}<{bitwidth}> = {zero};
      for (let j: ubit<{index_size1}> = 0..{size1}) {{ 
        {op.name}_expsum += exp({op.name}[i][j]); 
      }}
      for (let k: ubit<{index_size1}> = 0..{size1}) {{ 
        {res.name}[i][k] := exp({op.name}[i][k]); 
        ---
        {res.name}[i][k] := {res.name}[i][k] / {op.name}_expsum;
      }}
    }}
    """
    return LowerDahliaProgramToFuTIL(function, program_body, import_exp)


def max_pool2d(function):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.max_pool2d"""
    data, res = function.inputs[0].primitive, function.output.primitive

    strides = function.attributes.get_int_tuple("strides")
    pool_size = function.attributes.get_int_tuple("pool_size")
    layout = function.attributes.get_str("layout")
    ceil_mode = function.attributes.get_int("ceil_mode")
    assert layout == 'NCHW', f"Layout \'{layout}\' is not currently supported for nn.max_pool2d; please use `NCHW`"
    assert ceil_mode == False, "`ceil_mode` is not currently supported for nn.max_pool2d"
    bitwidth, data_type = data.data[0], data.data_type
    size0, size1, size2, size3 = res.data[1], res.data[2], res.data[3], res.data[4]

    program_body = f"""
    for (let b: ubit<32> = 0..{size0}) {{
      for (let c: ubit<32> = 0..{size1}) {{
        for (let y: ubit<32> = 0..{size2}) {{
          for (let x: ubit<32> = 0..{size3}) {{
            let stride_y: ubit<32> = y * {strides[0]}/*strides[0]*/;
            let stride_x: ubit<32> = x * {strides[1]}/*strides[1]*/;
            
            let max: {data_type}<{bitwidth}> = {data.name}[b][c][stride_y][stride_x];
            for (let m: ubit<32> = 0..{pool_size[0]}/*pool_size[0]*/) {{
              for (let n: ubit<32> = 0..{pool_size[1]}/*pool_size[1]*/) {{
                let pool_y: ubit<32> = stride_y + m;
                let pool_x: ubit<32> = stride_x + n;
                let current: {data_type}<{bitwidth}> = {data.name}[b][c][pool_y][pool_x];
                if (current > max) {{ max := current; }} 
              }}
            }}
            {res.name}[b][c][y][x] := max;
          }} 
        }} 
      }} 
    }} 
    """
    return LowerDahliaProgramToFuTIL(function, program_body)


# Only supports a small subset of the `conv2d` function. For example,
# dilation and grouped convolution are not supported.
def conv2d(function):
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.conv2d"""
    data, weight, res = function.inputs[0].primitive, function.inputs[1].primitive, function.output.primitive

    strides = function.attributes.get_int_tuple("strides")
    kernel_size = function.attributes.get_int_tuple("kernel_size")
    channels = function.attributes.get_int("channels")
    bitwidth, data_type = data.data[0], data.data_type
    size0, size1, size2, size3 = res.data[1], res.data[2], res.data[3], res.data[4]

    zero = '0.0' if data_type == 'ufix' or data_type == 'fix' else '0'
    program_body = f"""
    for (let b: ubit<32> = 0..{size0}) {{
      for (let c: ubit<32> = 0..{size1}) {{
        for (let y: ubit<32> = 0..{size2}) {{
          for (let x: ubit<32> = 0..{size3}) {{
            let sum: {data_type}<{bitwidth}> = {zero};
            
            for (let k: ubit<32> = 0..{channels}) {{
              for (let dy: ubit<32> = 0..{kernel_size[1]}/*kernel_size[1]*/) {{
                for (let dx: ubit<32> = 0..{kernel_size[0]}/*kernel_size[0]*/) {{
                  let kernel_y: ubit<32> = (/*strides[0]*/{strides[0]} * y) + dy;
                  let kernel_x: ubit<32> = (/*strides[1]*/{strides[1]} * x) + dx;     
                }} combine {{ sum += {data.name}[b][k][kernel_y][kernel_x] * {weight.name}[c][k][dy][dx]; }}
              }}
            }}
            {res.name}[b][c][y][x] := sum;
          }} 
        }} 
      }} 
    }} 
    """
    return LowerDahliaProgramToFuTIL(function, program_body)


# Mapping from Relay function names to their respective Dahlia lowering.
RelayCallNodes = {
    'nn.dense': dense,
    'nn.batch_flatten': batch_flatten,
    'nn.batch_matmul': batch_matmul,
    'nn.bias_add': bias_add,
    'nn.relu': relu, 'nn.softmax': softmax,
    'nn.max_pool2d': max_pool2d,
    'nn.conv2d': conv2d,
    'negative': negative,
    'expand_dims': expand_dims,
    'sqrt': sqrt
}

# Mapping from Relay binary calls to the respective Dahlia operator.
BuiltInBinaryOps = {'add': '+', 'divide': '/', 'multiply': '*', 'subtract': '-'}


def GetRelayFunctionCall(function_name) -> RelayFunctionCall:
    """
    Returns the corresponding name, function, and `op` type (if it is a binary op, otherwise None)
    of the Relay function call. If the function call isn't supported, fails with an assertion.
    """
    function = name = op = None
    assert function_name in BuiltInBinaryOps or function_name in RelayFunctionCalls, \
        f'{function_name} is not supported for lowering from Relay IR to FuTIL.'
    if function_name in BuiltInBinaryOps:
        op = BuiltInBinaryOps[function_name]
        function = broadcast
        name = function_name
    else:
        function = RelayFunctionCalls[function_name]
        name = function.__name__
    return function, name, op
