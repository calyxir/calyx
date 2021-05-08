from typing import List
from calyx.py_ast import *
from dahlia_utils import *
from calyx.gen_exp import generate_exp_taylor_series_approximation


### Dahlia Implementations for Relay Call Nodes ###

# Context: While implementing a Relay frontend for
# Calyx, we decided it would be easier to implement
# the Relay calls, e.g. `nn.softmax`, in Dahlia, and
# then lower the corresponding function definition
# to a Calyx program. Perhaps one day these will
# be replaced directly with Calyx components.
#
# In some cases, there is an effort to allow certain
# functions take on varying dimensionality, which
# trades off code readability for minimizing duplication.
#
# Local variables declared in each Dahlia implementation
# should use the `__` prefix, e.g. `x` should be named `__x`
# to avoid name collisions.


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
        current_dimension = f"[__{index_var}]"
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

def dropout(fd: DahliaFuncDef) -> str:
    """https://tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.dropout"""
    p = fd.attributes.get_str("rate")
    data, res = fd.args[0], fd.dest
    data_type = fd.data_type
    num_dims = get_dims(res.comp)
    inverse_rate = 1 / (1 - p)

    indices = []
    var_name = CHARACTER_I
    for n in range(num_dims):
        # Determine loop body indices.
        indices.append(f"[__{var_name}]")
        var_name = next_character(var_name)
    indices = ''.join(indices)

    loop_body = f"{res.id.name}{indices} := {data.id.name}{indices} * ({inverse_rate} as {data_type});"
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
        index = f"[__{var_name}]"
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
        indices += f"[__{var_name}]"
        var_name = next_character(var_name)

    zero = f"(0.0 as {fd.data_type})" if "fix" in fd.data_type else "0"
    loop_body = f"{res.id.name}{indices} := {zero} - {inp.id.name}{indices};"
    return emit_dahlia_definition(fd, emit_dahlia_loop(res, loop_body))


def batch_flatten(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.batch_flatten"""
    data, res = fd.args[0], fd.dest

    var_name = CHARACTER_I
    args = data.comp.args
    data_indices = ""
    res_indices = f"[__{var_name}]"
    num_dims = get_dims(data.comp)
    for i in range(num_dims):
        index = f"[__{var_name}]"
        data_indices += index
        var_name = next_character(var_name)

    var_name = f"__{var_name}"
    res_indices += f"[{var_name}]"

    loop_body = f"""{res.id.name}{res_indices} := \
           {data.id.name}{data_indices}; \
           {var_name} := {var_name} + 1;"""

    return emit_dahlia_definition(
        fd,
        (
            # Uses the index after the batch dimension.
            f"let {var_name}: ubit<{res.comp.args[4]}> = 0;",
            emit_dahlia_loop(data, loop_body),
        ),
    )


def bias_add(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.bias_add"""
    data, bias, res = fd.args[0], fd.args[1], fd.dest
    axis_attribute = fd.attributes.get_int("axis")
    num_dims = get_dims(data.comp)
    axis = num_dims - 1 if axis_attribute == -1 else axis_attribute

    var_name = CHARACTER_I
    data_indices = ""
    args = data.comp.args
    for i in range(num_dims):
        # Determine loop body indices based on `axis` provided.
        size = args[i + 1]
        index_size = args[i + 1 + num_dims]
        index = f"[__{var_name}]"
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
        nn.max_pool2d; please use `NCHW`."""
    assert (
        ceil_mode == False
    ), "`ceil_mode` is not currently supported for nn.max_pool2d"

    args = res.comp.args
    width = args[0]
    data_type = fd.data_type
    size0, size1, size2, size3 = args[1:5]

    return emit_dahlia_definition(
        fd,
        f"""for (let __b: ubit<{width}> = 0..{size0}) {{
          for (let __c: ubit<{width}> = 0..{size1}) {{
            for (let __y: ubit<{width}> = 0..{size2}) {{
              for (let __x: ubit<{width}> = 0..{size3}) {{
                let __stride_y: ubit<{width}> = __y * {strides[0]}/*strides[0]*/;
                let __stride_x: ubit<{width}> = __x * {strides[1]}/*strides[1]*/;

                let __max: {data_type} = {data.id.name}[__b][__c][__stride_y][__stride_x];
                for (let __m: ubit<{width}> = 0..{pool_size[0]}/*pool_size[0]*/) {{
                  for (let __n: ubit<{width}> = 0..{pool_size[1]}/*pool_size[1]*/) {{
                    let __pool_y: ubit<{width}> = __stride_y + __m;
                    let __pool_x: ubit<{width}> = __stride_x + __n;
                    let __current: {data_type} = {data.id.name}[__b][__c][__pool_y][__pool_x];
                    if (__current > __max) {{ __max := __current; }}
                  }}
                }}
                {res.id.name}[__b][__c][__y][__x] := __max;
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
        indices += f"[__{var_name}]"
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
        indices += f"[__{var_name}]"
        var_name = next_character(var_name)

    loop_body = f"""let __tmp = sqrt({data.id.name}{indices});
                    {res.id.name}{indices} := __tmp;"""
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
        f"""let __transpose_{b.id.name}: {type}[{M2_size0}][{M2_size2}][{M2_size1}];
        for (let __batch: ubit<{M1_index_size0}> = 0..{M1_size0}) {{
          for (let __i: ubit<{M2_index_size1}> = 0..{M2_size1}) {{
            for (let __j: ubit<{M2_index_size2}> = 0..{M2_size2}) {{
              __transpose_{b.id.name}[__batch][__j][__i] := {b.id.name}[__batch][__i][__j];
            }}
          }}
        }}
        ---
        for (let __batch: ubit<{M1_index_size0}> = 0..{M1_size0}) {{
          for (let __i: ubit<{M1_index_size1}> = 0..{M1_size1}) {{
            for (let __j: ubit<{M2_index_size1}> = 0..{M2_size1}) {{
              for (let __k: ubit<{M2_index_size2}> = 0..{M2_size2}) {{
                let __product = {a.id.name}[__batch][__i][__k] * __transpose_{b.id.name}[__batch][__k][__j];
              }} combine {{ {res.id.name}[__batch][__i][__j] += __product; }}
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
        f"""let __transpose_{b.id.name}: {type}[{M2_size1}][{M2_size0}];
        for (let __i: ubit<{M2_index_size0}> = 0..{M2_size0}) {{
          for (let __j: ubit<{M2_index_size1}> = 0..{M2_size1}) {{
            __transpose_{b.id.name}[__j][__i] := {b.id.name}[__i][__j];
          }}
        }}
        for (let __i: ubit<{M1_index_size0}> = 0..{M1_size0}) {{
          for (let __j: ubit<{M2_index_size0}> = 0..{M2_size0}) {{
            for (let __k: ubit<{M1_index_size1}> = 0..{M1_size1}) {{
              let __product = {a.id.name}[__i][__k] * __transpose_{b.id.name}[__k][__j];
            }} combine {{ {res.id.name}[__i][__j] += __product; }}
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
    size0, size1, size2, size3 = res.comp.args[1:5]

    channels = fd.attributes.get_int("channels")
    if channels is None:
        channels = size1

    return emit_dahlia_definition(
        fd,
        f"""for (let __b: ubit<32> = 0..{size0}) {{
          for (let __c: ubit<32> = 0..{size1}) {{
            for (let __y: ubit<32> = 0..{size2}) {{
              for (let __x: ubit<32> = 0..{size3}) {{
                let __sum: {data_type} = {'0.0' if 'fix' in data_type else '0'};

                for (let __k: ubit<32> = 0..{channels}) {{
                  for (let __dy: ubit<32> = 0..{kernel_size[1]}/*kernel_size[1]*/) {{
                    for (let __dx: ubit<32> = 0..{kernel_size[0]}/*kernel_size[0]*/) {{
                      let __kernel_y: ubit<32> = (/*strides[0]*/{strides[0]} * __y) + __dy;
                      let __kernel_x: ubit<32> = (/*strides[1]*/{strides[1]} * __x) + __dx;
                    }} combine {{
                      __sum += {data.id.name}[__b][__k][__kernel_y][__kernel_x] *
                             {weight.id.name}[__c][__k][__dy][__dx];
                    }}
                  }}
                }}
                {res.id.name}[__b][__c][__y][__x] := __sum;
              }} 
            }} 
          }} 
        }} 
        """,
    )

def reshape(fd: DahliaFuncDef) -> str:
    """https://tvm.apache.org/docs/api/python/relay/index.html#tvm.relay.reshape"""
    data, res = fd.args[0], fd.dest
    newshape = fd.attributes.get_int_tuple("newshape")
    ddims = get_dims(data.comp)
    rdims = get_dims(res.comp)

    assert (
        newshape[0] == -1 and rdims == 2
    ), f"Only supports a subset of `reshape` functionality (where the dimensions are inferred)."

    data_indices, res_indices = "", ""
    var_name = CHARACTER_I
    for _ in range(ddims):
        data_indices += f"[__{var_name}]"
        var_name = next_character(var_name)

    data_type = fd.data_type
    input = f"{data.id.name}{data_indices}"
    result = f"{res.id.name}[0][__m]"
    loop_body = f"""{result} := {input}; __m += (1 as ubit<{res.comp.args[4]}>);"""
    program = (
        f"let __m: ubit<{res.comp.args[4]}> = 0;",
        emit_dahlia_loop(data, loop_body)
    )
    return emit_dahlia_definition(fd, program)


def softmax(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.softmax"""
    data, res = fd.args[0], fd.dest
    axis = fd.attributes.get_int("axis")
    assert axis == -1 or axis == 1, f"nn.softmax with axis = {axis} is not supported."

    data_type = fd.data_type
    size0, size1, index_size0, index_size1 = data.comp.args[1:5]

    return emit_dahlia_definition(
        fd,
        f"""
        let __max :{data_type} = {data.id.name}[0][0];
        for (let __i: ubit<{index_size0}> = 0..{size0}) {{
          for (let __j: ubit<{index_size1}> = 0..{size1}) {{
            if ({data.id.name}[__i][__j] > __max) {{ __max := {data.id.name}[__i][__j]; }}
          }}
        }}
        for (let __i: ubit<{index_size0}> = 0..{size0}) {{
          let __exp_sum: {data_type} = {'0.0' if 'fix' in data_type else '0'};
          for (let __j: ubit<{index_size1}> = 0..{size1}) {{
            let __t0 = {data.id.name}[__i][__j] - __max;
            let __t1 = exp(__t0);
            __exp_sum += __t1;
          }}
          for (let __k: ubit<{index_size1}> = 0..{size1}) {{
            let __t2 = {data.id.name}[__i][__k] - __max;
            let __t3 = exp(__t2);
            {res.id.name}[__i][__k] := __t3 / __exp_sum; 
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
    "dropout": dropout,
    "reshape": reshape,
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
        f"""import futil("primitives/math.futil")
        {{
          def exp(x: {type}): {type};
          def sqrt(in: {type}): {type};
          def fp_sqrt(in: {type}): {type};
        }}"""
    ]

    exp_components = ""
    if any(f.function_id == "softmax" for f in func_defs):
        # Import `exp` operator for softmax.
        sep = type.find(",")
        width = int(type[type.find("<") + 1 : sep])
        int_width = int(type[sep + 1 : type.find(">")])
        exp_components = generate_exp_taylor_series_approximation(
            degree=8,
            width=width,
            int_width=int_width,
            is_signed="u" not in type,
        )
        exp_components = "\n".join(c.doc() for c in exp_components)

    return dahlia_to_calyx(imports, dahlia_definitions) + exp_components
