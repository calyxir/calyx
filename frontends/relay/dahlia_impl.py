from typing import List
from calyx.ast import *
from dahlia_utils import *


####################################################################################################
################## Dahlia Implementations for Relay Call Nodes #####################################
####################################################################################################


def broadcast(fd: DahliaFuncDef) -> str:
    """Implements array broadcasting:
    Two dimensions are compatible when either (1) they're equal,
    or (2) one of them is `1`. It is not required that both
    operands have the same number of dimensions either. When
    lowering from Relay IR, we are guaranteed the arrays are
    compatible for broadcasting.

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

    Reference: https://numpy.org/doc/stable/user/basics.broadcasting.html
    """
    op1, op2, res = fd.args[0], fd.args[1], fd.dest
    op1_dims, op2_dims, res_dims = (
        get_dims(op1.comp),
        get_dims(op2.comp),
        get_dims(res.comp),
    )

    # Get memory sizes in reversed order.
    op1_sizes, op2_sizes, res_sizes = [], [], []
    for i in reversed(range(0, op1_dims)):
        op1_sizes.append(op1.comp.args[i + 1])
    for i in reversed(range(0, op2_dims)):
        op2_sizes.append(op2.comp.args[i + 1])
    for i in reversed(range(0, res_dims)):
        res_sizes.append(res.comp.args[i + 1])

    # Gets the last variable name for indexing, since
    # we will compare sizes in the reverse direction.
    index_var = chr(ord(CHARACTER_I) + res_dims - 1)

    # Determine the value address value at each index.
    # This will either be a variable name or `0`.
    index_zero = "[0]"
    op1_indices, op2_indices, res_indices = [], [], []
    for i in range(0, len(res_sizes)):
        current_dimension = f"[{index_var}]"
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
        else:
            # op2_sizes[i] < op1_sizes[i]
            op1_indices.append(index_zero)
            op2_indices.append(current_dimension)
        index_var = next_character(index_var, -1)

    # Resulting index in the nested for loop,
    # e.g. for `op1[i][j][0][k]`, this is `[i][j][0][k]`.
    op1_index = "".join(reversed(op1_indices))
    op2_index = "".join(reversed(op2_indices))
    res_index = "".join(reversed(res_indices))
    loop_body = (
        f"{res.id.name}{res_index} := {op1.id.name}{op1_index} "
        f"{BinaryOps[fd.function_id]} {op2.id.name}{op2_index};"
    )

    return emit_dahlia_definition(fd, emit_dahlia_loop(res, loop_body))


# https://github.com/cucapra/calyx/issues/401
# Please read the issue above before trying
# to lower this using `relay.fromtext`.
def expand_dims(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/index.html#tvm.relay.expand_dims"""
    axis = fd.attributes.get_int("axis")
    data, res = fd.args[0], fd.dest

    res_indices, data_indices = "", ""
    var_name = CHARACTER_I
    num_dims = get_dims(data.comp)
    for n in range(num_dims):
        # Determine loop body indices.
        index = f"[{var_name}]"
        res_indices += index
        data_indices += index
        if axis == n + 1:
            # Append expanded dimensions.
            res_indices += "[0]" * fd.attributes.get_int("num_newaxis")
        var_name = next_character(var_name)

    loop_body = f"{res.id.name}{res_indices} := {data.id.name}{data_indices};"
    return emit_dahlia_definition(fd, emit_dahlia_loop(data, loop_body))


def negative(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/index.html#tvm.relay.negative"""
    inp, res = fd.args[0], fd.dest

    var_name = CHARACTER_I
    indices = ""
    num_dims = get_dims(inp.comp)
    for _ in range(num_dims):
        # Determine loop body indices.
        indices += f"[{var_name}]"
        var_name = next_character(var_name)

    zero = "0.0" if fd.data_type == "fix" else "0"
    loop_body = f"{res.id.name}{indices} := {zero} - {inp.id.name}{indices};"
    return emit_dahlia_definition(fd, emit_dahlia_loop(res, loop_body))


def batch_flatten(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.batch_flatten"""
    data, res = fd.args[0], fd.dest

    var_name = CHARACTER_I
    args = data.comp.args
    data_indices = ""
    res_indices = f"[{var_name}]"
    num_dims = get_dims(data.comp)
    for i in range(num_dims):
        index = f"[{var_name}]"
        data_indices += index
        var_name = next_character(var_name)

    res_indices += f"[{var_name}]"

    loop_body = f"""{res.id.name}{res_indices} := \
           {data.id.name}{data_indices}; \
           {var_name} := {var_name} + 1;"""
    return emit_dahlia_definition(
        fd,
        (
            # We use args[3] because the output is
            # 2-dimensional (batch). Therefore, we want
            # the second index size in the memory.
            f"let {var_name}: ubit<{args[3]}> = 0;",
            emit_dahlia_loop(data, loop_body),
        ),
    )


def bias_add(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.bias_add"""
    data, bias, res = fd.args[0], fd.args[1], fd.dest
    axis_attribute = fd.attributes.get_int("axis")
    axis = num_dims - 1 if axis_attribute == -1 else axis_attribute

    var_name = CHARACTER_I
    data_indices = ""
    args = data.comp.args
    num_dims = get_dims(data.comp)
    for i in range(num_dims):
        # Determine loop body indices based on `axis` provided.
        size = args[i + 1]
        index_size = args[i + 1 + num_dims]
        index = f"[{var_name}]"
        if axis == i:
            # Determine which `var_name` is
            # associated with the bias index.
            bias_index = index

        data_indices += index
        var_name = next_character(var_name)
    loop_body = f"""{res.id.name}{data_indices} :=
                {data.id.name}{data_indices} + {bias.id.name}{bias_index};"""

    return emit_dahlia_definition(fd, emit_dahlia_loop(data, loop_body))


def max_pool2d(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.max_pool2d"""
    data, res = fd.args[0], fd.dest

    strides = fd.attributes.get_int_tuple("strides")
    pool_size = fd.attributes.get_int_tuple("pool_size")
    layout = fd.attributes.get_str("layout")
    ceil_mode = fd.attributes.get_int("ceil_mode")
    assert (
            layout == "NCHW"
    ), f"""Layout \'{layout}\' is not currently supported for
        nn.max_pool2d; please use `NCHW`"""
    assert (
            ceil_mode == False
    ), "`ceil_mode` is not currently supported for nn.max_pool2d"

    args = res.comp.args
    width = args[0]
    data_type = fd.data_type
    size0, size1, size2, size3 = args[1:5]

    return emit_dahlia_definition(
        fd,
        f"""for (let b: ubit<{width}> = 0..{size0}) {{
          for (let c: ubit<{width}> = 0..{size1}) {{
            for (let y: ubit<{width}> = 0..{size2}) {{
              for (let x: ubit<{width}> = 0..{size3}) {{
                let stride_y: ubit<{width}> = y * {strides[0]}/*strides[0]*/;
                let stride_x: ubit<{width}> = x * {strides[1]}/*strides[1]*/;

                let max: {data_type} = {data.id.name}[b][c][stride_y][stride_x];
                for (let m: ubit<{width}> = 0..{pool_size[0]}/*pool_size[0]*/) {{
                  for (let n: ubit<{width}> = 0..{pool_size[1]}/*pool_size[1]*/) {{
                    let pool_y: ubit<{width}> = stride_y + m;
                    let pool_x: ubit<{width}> = stride_x + n;
                    let current: {data_type} = {data.id.name}[b][c][pool_y][pool_x];
                    if (current > max) {{ max := current; }}
                  }}
                }}
                {res.id.name}[b][c][y][x] := max;
              }} 
            }} 
          }} 
        }} 
        """,
    )


def relu(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.relu"""
    data, res = fd.args[0], fd.dest
    num_dims = get_dims(data.comp)
    args = data.comp.args

    indices = ""
    var_name = CHARACTER_I
    for _ in range(num_dims):
        indices += f"[{var_name}]"
        var_name = next_character(var_name)

    data_type = fd.data_type
    zero = f'({"0.0" if "fix" in data_type else "0"} as {data_type})'
    input = f"{data.id.name}{indices}"
    result = f"{res.id.name}{indices}"
    loop_body = f"""if ({input} > {zero}) {{ {result} := {input}; }} 
                    else {{ {result} := {zero}; }}"""

    return emit_dahlia_definition(fd, emit_dahlia_loop(data, loop_body))


def sqrt(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/index.html#tvm.relay.sqrt"""
    data, res = fd.args[0], fd.dest
    num_dims = get_dims(data.comp)
    args = data.comp.args

    indices = ""
    var_name = CHARACTER_I
    for _ in range(num_dims):
        indices += f"[{var_name}]"
        var_name = next_character(var_name)

    loop_body = f"""let tmp = sqrt({data.id.name}{indices});
                    {res.id.name}{indices} := tmp;"""
    return emit_dahlia_definition(fd, emit_dahlia_loop(data, loop_body))


def batch_matmul(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.batch_matmul"""
    a, b, res = fd.args[0], fd.args[1], fd.dest
    type = fd.data_type
    bitwidth, M1_size0, M1_size1, M1_size2 = a.comp.args[0:4]
    M1_index_size0, M1_index_size1, M1_index_size2 = a.comp.args[4:7]

    M2_size0, M2_size1, M2_size2 = b.comp.args[1:4]
    M2_index_size0, M2_index_size1, M2_index_size2 = b.comp.args[4:7]

    return emit_dahlia_definition(
        fd,
        f"""let transpose_{b.id.name}: {type}[{M2_size0}][{M2_size2}][{M2_size1}];
        for (let batch: ubit<{M1_index_size0}> = 0..{M1_size0}) {{
          for (let i: ubit<{M2_index_size1}> = 0..{M2_size1}) {{
            for (let j: ubit<{M2_index_size2}> = 0..{M2_size2}) {{
              transpose_{b.id.name}[batch][j][i] := {b.id.name}[batch][i][j];
            }}
          }}
        }}
        for (let batch: ubit<{M1_index_size0}> = 0..{M1_size0}) {{
          for (let i: ubit<{M1_index_size1}> = 0..{M1_size1}) {{
            for (let j: ubit<{M2_index_size1}> = 0..{M2_size1}) {{
              for (let k: ubit<{M2_index_size2}> = 0..{M2_size2}) {{
                let product = {a.id.name}[batch][i][k] * transpose_{b.id.name}[batch][k][j];
              }} combine {{ {res.id.name}[batch][i][j] += product; }}
            }}
          }}
        }}
        """,
    )


def dense(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.dense"""
    a, b, res = fd.args[0], fd.args[1], fd.dest
    type = fd.data_type
    M1_size0, M1_size1 = a.comp.args[1:3]
    M1_index_size0, M1_index_size1 = a.comp.args[3:5]
    M2_size0, M2_size1, M2_index_size0, M2_index_size1 = b.comp.args[1:5]

    return emit_dahlia_definition(
        fd,
        f"""let transpose_{b.id.name}: {type}[{M2_size1}][{M2_size0}];
        for (let i: ubit<{M2_index_size0}> = 0..{M2_size0}) {{
          for (let j: ubit<{M2_index_size1}> = 0..{M2_size1}) {{
            transpose_{b.id.name}[j][i] := {b.id.name}[i][j];
          }}
        }}
        for (let i: ubit<{M1_index_size0}> = 0..{M1_size0}) {{
          for (let j: ubit<{M2_index_size0}> = 0..{M2_size0}) {{
            for (let k: ubit<{M1_index_size1}> = 0..{M1_size1}) {{
              let product = {a.id.name}[i][k] * transpose_{b.id.name}[k][j];
            }} combine {{ {res.id.name}[i][j] += product; }}
          }}
        }}
        """,
    )


def conv2d(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.conv2d"""
    data, weight, res = fd.args[0], fd.args[1], fd.dest
    data_type = fd.data_type

    strides = fd.attributes.get_int_tuple("strides")
    kernel_size = fd.attributes.get_int_tuple("kernel_size")
    channels = fd.attributes.get_int("channels")

    size0, size1, size2, size3 = res.comp.args[1:5]

    return emit_dahlia_definition(
        fd,
        f"""for (let b_: ubit<32> = 0..{size0}) {{
          for (let c_: ubit<32> = 0..{size1}) {{
            for (let y_: ubit<32> = 0..{size2}) {{
              for (let x_: ubit<32> = 0..{size3}) {{
                let sum: {data_type} =
                         {'0.0' if 'fix' in data_type else '0'};

                for (let k: ubit<32> = 0..{channels}) {{
                  for (let dy: ubit<32> = 0..{kernel_size[1]}/*kernel_size[1]*/) {{
                    for (let dx: ubit<32> = 0..{kernel_size[0]}/*kernel_size[0]*/) {{
                      let kernel_y: ubit<32> = (/*strides[0]*/{strides[0]} * y_) + dy;
                      let kernel_x: ubit<32> = (/*strides[1]*/{strides[1]} * x_) + dx;
                    }} combine {{
                      sum += {data.id.name}[b_][k][kernel_y][kernel_x] *
                             {weight.id.name}[c_][k][dy][dx];
                    }}
                  }}
                }}
                {res.id.name}[b_][c_][y_][x_] := sum;
              }} 
            }} 
          }} 
        }} 
        """,
    )


def softmax(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.softmax"""
    data, res = fd.args[0], fd.dest
    axis = fd.attributes.get_int("axis")
    assert axis == -1 or axis == 1, f"nn.softmax with axis = {axis} is not supported."

    data_type = fd.data_type
    size0, size1, index_size0, index_size1 = data.comp.args[1:5]

    # The value of `e` if Q = 32.16, otherwise `3`.
    e = "13044242090" if "fix" in data_type else "3"

    return emit_dahlia_definition(
        fd,
        f"""let e: {data_type} = {e};
        for (let i: ubit<{index_size0}> = 0..{size0}) {{
          let {data.id.name}_expsum: {data_type} =
              {'0.0' if 'fix' in data_type else '0'};

          for (let j: ubit<{index_size1}> = 0..{size1}) {{
            let tmp1 = exp(e, {data.id.name}[i][j]);
            {data.id.name}_expsum += tmp1;
          }}
          for (let k: ubit<{index_size1}> = 0..{size1}) {{
            let tmp2 = exp(e, {data.id.name}[i][k]);
            {res.id.name}[i][k] := tmp2;
            {res.id.name}[i][k] :=
            {res.id.name}[i][k] / {data.id.name}_expsum;
          }}
        }}""",
    )


# Mapping from Relay function names to their respective Dahlia lowering.
RelayCallNodes = {
    "expand_dims": expand_dims,
    "negative": negative,
    "batch_flatten": batch_flatten,
    "batch_matmul": batch_matmul,
    "bias_add": bias_add,
    "conv2d": conv2d,
    "dense": dense,
    "max_pool2d": max_pool2d,
    "relu": relu,
    "softmax": softmax,
    "sqrt": sqrt,
}

# Mapping from Relay binary calls to
# the respective Dahlia operator.
BinaryOps = {"add": "+", "divide": "/", "multiply": "*", "subtract": "-"}


def emit_components(func_defs: List[DahliaFuncDef]) -> str:
    """Returns a string containing all the components
    created from the list of Dahlia function definitions.
    This does not include the import statement.
    """
    if not func_defs:
        return ""

    dahlia_definitions = []
    for func_def in func_defs:
        id = func_def.function_id
        assert (
                id in RelayCallNodes or id in BinaryOps
        ), f"{id} not supported for lowering."

        # If the function is a binary operation, use broadcasting.
        # Otherwise, use the associated Relay function.
        apply = broadcast if id in BinaryOps else RelayCallNodes[id]
        dahlia_definitions.append(apply(func_def))

    type = func_defs[0].data_type
    imports = [
        f"""import futil("primitives/bitnum/math.futil")
        {{
          def exp(base: {type}, exp: {type}): {type};
          def sqrt(in: {type}): {type};
        }}"""
    ]

    return dahlia_to_futil(
        "\n".join(imports + dahlia_definitions)
    )
