from typing import List
from calyx.py_ast import *
from dahlia_utils import *
from calyx.gen_exp import generate_exp_taylor_series_approximation, generate_fp_pow_full
from calyx.utils import float_to_fixed_point
from calyx.builder import Builder
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
    indices = "".join(indices)

    loop_body = f"{res.id.name}{indices} := {data.id.name}{indices} * ({inverse_rate} as {data_type});"
    return emit_dahlia_definition(fd, emit_dahlia_loop(res, loop_body))


# https://github.com/calyxir/calyx/issues/401
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

    sqrt_op = "fp_sqrt" if "fix" in fd.data_type else "sqrt"

    indices = ""
    var_name = CHARACTER_I
    for _ in range(num_dims):
        indices += f"[__{var_name}]"
        var_name = next_character(var_name)

    loop_body = f"""let __tmp = {sqrt_op}({data.id.name}{indices});
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


def dense(fd: DahliaFuncDef, save_mem=True) -> str:
    """
    tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.dense
    If save_mem=True, instead of actually building the transpose of the weight matrix,
    we just access W[j][i] everytime we would have accessed W^T[i][j]. It seems
    to be a better way (in terms of resource usage) to calculate dense, which
    is why it has been save_mem is the default setting.
    """
    a, b, res = fd.args[0], fd.args[1], fd.dest
    type = fd.data_type
    M1_size0, M1_size1, M1_index_size0, M1_index_size1 = a.comp.args[1:5]
    M2_size0, M2_size1, M2_index_size0, M2_index_size1 = b.comp.args[1:5]
    units = fd.attributes.get_int("units")
    assert units is None or units == res.comp.args[2], (
        "parameter for `units` should be the same as the second dimension of the result")
    if save_mem:
        # don't generate internal `transpose` memory
        return emit_dahlia_definition(
            fd,
            f"""
          for (let __i: ubit<{M1_index_size0}> = 0..{M1_size0}) {{
            for (let __j: ubit<{M2_index_size0}> = 0..{M2_size0}) {{
              for (let __k: ubit<{M1_index_size1}> = 0..{M1_size1}) {{
                let __product = {a.id.name}[__i][__k] * {b.id.name}[__j][__k];
              }} combine {{ {res.id.name}[__i][__j] += __product; }}
            }}
          }}
          """,
        )
    else:
        # generate internal `transpose` memory.
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
    data_size0, data_size1, data_size2, data_size3 = data.comp.args[1:5]
    data_type = fd.data_type
    strides = fd.attributes.get_int_tuple("strides")
    kernel_size = fd.attributes.get_int_tuple("kernel_size")
    padding = fd.attributes.get_int_tuple("padding")
    add_padding = True
    if max(padding) == min(padding) and max(padding) == 0:
        add_padding = False
        prepend_rows = 0
        prepend_cols = 0
    else:
        assert len(
            padding) == 4, "Can only handle when we're given 4 padding values"
        prepend_rows = padding[0]
        # might want to use this value to check when out of bounds on the high end
        # currently if index is too high, it just deafults to a 0 value.
        append_rows = padding[1]
        prepend_cols = padding[2]
        # might want to use this value to check when out of bounds on the high end
        # currently when index is too high, it just defaults to 0 value
        append_cols = padding[3]

    # can generalize these numbers based on padding if necessary
    dim2_lowest = prepend_rows
    dim3_lowest = prepend_cols
    dim2_limit = data_size2 + prepend_rows
    dim3_limit = data_size3 + prepend_cols
    size0, size1, size2, size3 = res.comp.args[1:5]

    # to handle padding. Right now we hard code, but we can change the code
    # to be more general if necessary.
    assign_tensor_val = (
        f"""// our code is "simulating" the padding of the input array
                      let __padded_tensor_val: {data_type} = {'0.0' if 'fix' in data_type else '0'};
                      ---
                      if (__kernel_y >= {dim2_lowest} && __kernel_y < {dim2_limit} && __kernel_x >= {dim3_lowest} && __kernel_x < {dim3_limit}) {{
                        __padded_tensor_val := {data.id.name}[__b][__k][__kernel_y - {prepend_rows}][__kernel_x - {prepend_cols}];
                      }}"""
        if add_padding
        else f"""let __padded_tensor_val: {data_type} =  {data.id.name}[__b][__k][__kernel_y][__kernel_x];"""
    )

    # If no channels provided, inferred from the second dimension of the res (which
    # is size1 since we start counting from size0).
    output_channels = fd.attributes.get_int("channels") or size1

    input_channels = data_size1

    return emit_dahlia_definition(
        fd,
        f"""for (let __b: ubit<32> = 0..{size0}) {{
          for (let __c: ubit<32> = 0..{output_channels}) {{
            for (let __y: ubit<32> = 0..{size2}) {{
              for (let __x: ubit<32> = 0..{size3}) {{
                let __sum: {data_type} = {'0.0' if 'fix' in data_type else '0'};
                for (let __k: ubit<32> = 0..{input_channels}) {{
                  for (let __dy: ubit<32> = 0..{kernel_size[1]}/*kernel_size[1]*/) {{
                    for (let __dx: ubit<32> = 0..{kernel_size[0]}/*kernel_size[0]*/) {{
                      let __kernel_y: ubit<32> = (/*strides[0]*/{strides[0]} * __y) + __dy;
                      let __kernel_x: ubit<32> = (/*strides[1]*/{strides[1]} * __x) + __dx;
                      ---
                      {assign_tensor_val}
                      ---
                       __sum += __padded_tensor_val *
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
    size0, size1 = data.comp.args[1:3]

    assert rdims == 2, "can only support reshaping into a 2d array"

    assert (
        newshape[0] == -1
        or newshape[1] == -1
        or newshape[0] == 1
        or (
            ddims == 4
            and newshape[0] == data.comp.args[1]
            and newshape[1] == data.comp.args[2]
            and 1 == data.comp.args[3]
            and 1 == data.comp.args[4]
        )
    ), f"""Only supports a subset of `reshape` functionality (i.e. where the dimensions are inferred).
        E.g.
        let  %x: Tensor[(1, 2, 2, 2), float32] = ...;
        let %x1: Tensor[(1, 8), float32] = reshape(%x, newshape[-1, 8]);

        Or supports reshape when the first dimension of the new size is 1

        Or supports reshape when all you are going from a 4d to 2d array, but the
        first two dimension sizes are the same.
        E.g.
        let  %x: Tensor[(4, 6, 1, 1), float32] = ...;
        let %x1: Tensor[(4, 6), float32] = reshape(%x, newshape[4, 6]);
        ---
        [Actual] newshape[0]: {newshape[0]},newshape[1]: {newshape[1]}, rdims: {rdims}
        """

    data_indices, res_indices = "", ""
    var_name = CHARACTER_I

    if newshape[0] == -1 or newshape[1] == -1 or newshape[0] == 1:
        for _ in range(ddims):
            data_indices += f"[__{var_name}]"
            var_name = next_character(var_name)
        input = f"{data.id.name}{data_indices}"
        result = f"{res.id.name}[0][__m]"
        loop_body = f"""{result} := {input}; __m += (1 as ubit<{res.comp.args[4]}>);"""
        program = (
            f"let __m: ubit<{res.comp.args[4]}> = 0;",
            emit_dahlia_loop(data, loop_body),
        )
        return emit_dahlia_definition(fd, program)
    else:
        for _ in range(2):
            data_indices += f"[__{var_name}]"
            var_name = next_character(var_name)
        input = f"{data.id.name}{data_indices}[0][0]"
        result = f"{res.id.name}{data_indices}"
        loop_body = f"""{result} := {input};"""
        return emit_dahlia_definition(fd, emit_dahlia_loop(data, loop_body))


def softmax(fd: DahliaFuncDef) -> str:
    """tvm.apache.org/docs/api/python/relay/nn.html#tvm.relay.nn.softmax"""
    data, res = fd.args[0], fd.dest
    axis = fd.attributes.get_int("axis")
    assert axis == - \
        1 or axis == 1, f"nn.softmax with axis = {axis} is not supported."

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


def clip(fd: DahliaFuncDef) -> str:
    """
    https://tvm.apache.org/docs/reference/api/python/relay/index.html
    Clips the data so it is all in between a_min and a_max
    """
    data, res = fd.args[0], fd.dest
    # getting a_min and a_max values
    a_min = fd.attributes.get_int("a_min")
    a_max = fd.attributes.get_int("a_max")
    ddims = get_dims(data.comp)

    data_indices, res_indices = "", ""
    data_type = fd.data_type
    var_name = CHARACTER_I
    for _ in range(ddims):
        data_indices += f"[__{var_name}]"
        res_indices += f"[__{var_name}]"
        var_name = next_character(var_name)

    input_indices = f"{data.id.name}{data_indices}"
    output = f"{res.id.name}{res_indices}"

    # any input val below a_min or above a_max should be altered to be
    # within the limit
    loop_body = f"""let val: {data_type} = {input_indices};
                if (val < ({a_min} as {data_type})) {{
                  {output} := ({a_min} as {data_type});
                }}
                else {{
                  if (val > ({a_max} as {data_type})){{
                    {output} := ({a_max} as {data_type});
                  }}
                  else{{
                    {output} := val;
                  }}
                }}
                """
    return emit_dahlia_definition(fd, emit_dahlia_loop(data, loop_body))


def concatenate(fd: DahliaFuncDef) -> str:
    """
    https://tvm.apache.org/docs/reference/api/python/relay/index.html
    """
    # data0 and data1 will be concatenated into res
    data0, data1, res = fd.args[0], fd.args[1], fd.dest
    # axis to concatenate along
    axis = fd.attributes.get_int("axis")
    rdims = get_dims(res.comp)

    # for (go thru data0):
    #     (assuming 4 dimensions for this ex.)
    #     res[i][j][k][l] = data0[i][j][k][l]
    var_name = CHARACTER_I
    indices0 = ""
    for _ in range(rdims):
        indices0 += f"[__{var_name}]"
        var_name = next_character(var_name)
    input0 = f"{data0.id.name}{indices0}"
    result0 = f"{res.id.name}{indices0}"
    loop0_body = f"""{result0} := {input0};"""
    loop0 = emit_dahlia_loop(data0, loop0_body)

    # for (go thru data1):
    #     (suppose 4 dimensions for this ex.)
    #     (suppose extending along axis = 0)
    #     res[i + len(data0[0])][j][k][l] = data0[i][j][k][l]
    res_indices1 = ""
    data_indices1 = ""
    var_name = CHARACTER_I
    d0_axis_len = data0.comp.args[axis + 1]
    for i in range(rdims):
        data_indices1 += f"[__{var_name}]"
        if i == axis:
            res_indices1 += f"[__{var_name} + " + str(d0_axis_len) + "]"
        else:
            res_indices1 += f"[__{var_name}]"
        var_name = next_character(var_name)
    input1 = f"{data1.id.name}{data_indices1}"
    result1 = f"{res.id.name}{res_indices1}"
    loop1_body = f"""{result1} := {input1};"""
    loop1 = emit_dahlia_loop(data1, loop1_body)

    # combinig the two loops in sequence
    loops = loop0 + "\n--- \n" + loop1

    return emit_dahlia_definition(fd, loops)


def avg_pool2d(fd: DahliaFuncDef) -> str:
    """
    https://tvm.apache.org/docs/reference/api/python/relay/nn.html
    Very similar to max_pool2d
    """
    data, res = fd.args[0], fd.dest
    strides = fd.attributes.get_int_tuple("strides")
    pool_size = fd.attributes.get_int_tuple("pool_size")
    layout = fd.attributes.get_str("layout")
    ceil_mode = fd.attributes.get_int("ceil_mode")
    assert (
        layout == "NCHW"
    ), f"""Layout \'{layout}\' is not currently supported for
        nn.avg_pool2d; please use `NCHW`."""
    assert (
        ceil_mode == False
    ), "`ceil_mode` is not currently supported for nn.avg_pool2d"

    args = res.comp.args
    width = args[0]
    data_type = fd.data_type
    size0, size1, size2, size3 = args[1:5]

    return emit_dahlia_definition(
        fd,
        f"""let __pool_area: {data_type} = ({pool_size[0]} as {data_type}) * ({pool_size[1]} as {data_type});
        for (let __b: ubit<{width}> = 0..{size0}) {{
          for (let __c: ubit<{width}> = 0..{size1}) {{
            for (let __y: ubit<{width}> = 0..{size2}) {{
              for (let __x: ubit<{width}> = 0..{size3}) {{
                let __stride_y: ubit<{width}> = __y * {strides[0]}/*strides[0]*/;
                let __stride_x: ubit<{width}> = __x * {strides[1]}/*strides[1]*/;

                let __total: {data_type} = {'0.0' if 'fix' in data_type else '0'};
                for (let __m: ubit<{width}> = 0..{pool_size[0]}/*pool_size[0]*/) {{
                  for (let __n: ubit<{width}> = 0..{pool_size[1]}/*pool_size[1]*/) {{
                    let __pool_y: ubit<{width}> = __stride_y + __m;
                    let __pool_x: ubit<{width}> = __stride_x + __n;
                    let __current: {data_type} = {data.id.name}[__b][__c][__pool_y][__pool_x];
                    __total := __total + __current;
                  }}
                }}
                let __avg: {data_type} = __total / __pool_area;
                {res.id.name}[__b][__c][__y][__x] := __avg;
              }}
            }}
          }}
        }}
        """,
    )


def global_avg_pool2d(fd: DahliaFuncDef) -> str:
    """
    https://tvm.apache.org/docs/reference/api/python/relay/nn.html#tvm.relay.nn.global_avg_pool2d
    """
    data, res = fd.args[0], fd.dest
    layout = fd.attributes.get_str("layout")
    assert (
        layout == "NCHW"
    ), f"""Layout \'{layout}\' is not currently supported for
        nn.global_avg_pool2d; please use `NCHW`."""

    args = res.comp.args
    data_args = data.comp.args
    width = args[0]
    data_type = fd.data_type
    size0, size1, size2, size3 = data_args[1:5]
    return emit_dahlia_definition(
        fd,
        f"""let __area: {data_type} = ({size2} as {data_type}) * ({size3} as {data_type});
        for (let __b: ubit<{width}> = 0..{size0}) {{
          for (let __c: ubit<{width}> = 0..{size1}) {{
            let __total: {data_type} = {'0.0' if 'fix' in data_type else '0'};
            for (let __m: ubit<{width}> = 0..{size2}) {{
              for (let __n: ubit<{width}> = 0..{size3}) {{
                 __total := __total + {data.id.name}[__b][__c][__m][__n];
              }}
            }}
            let __avg: {data_type} = __total / __area;
            {res.id.name}[__b][__c][0][0] := __avg;
          }}
        }}
        """,
    )


def lrn(fd: DahliaFuncDef) -> str:
    '''
    https://tvm.apache.org/docs/reference/api/python/relay/nn.html
    '''
    data, res = fd.args[0], fd.dest

    axis = fd.attributes.get_str("axis")

    assert (axis == 1), f"""currently can only support lrn along axis 1"""

    size = fd.attributes.get_str("size")

    bias = float_to_fixed_point(fd.attributes.get_str("bias"), 16)
    alpha = float_to_fixed_point(fd.attributes.get_str("alpha"), 16)
    beta = float_to_fixed_point(fd.attributes.get_str("beta"), 16)

    res_args = res.comp.args
    width = res_args[0]
    data_args = data.comp.args
    data_type = fd.data_type
    size0, size1, size2, size3 = data_args[1:5]

    assert size0 == 1, f"""currently only supports lrn if the first dimension of the tensor has size of 1"""

    return emit_dahlia_definition(
        fd,
        f"""for (let __n: ubit<{width}> = 0..{size0}) {{
          for (let __c: ubit<{width}> = 0..{size1}) {{
            for (let __h: ubit<{width}> = 0..{size2}) {{
              for (let __w: ubit<{width}> = 0..{size3}) {{
                let __sum: {data_type} = {'0.0' if 'fix' in data_type else '0'};
                for (let __i: ubit<{width}> = 0..{size-1}){{
                  let __c_index: ubit<{width}> = __c - (({size-1} as ubit<{width}>)/(2 as ubit<{width}>)) + __i;
                  if (__c_index >=0 && __c_index < {size1}){{
                      __sum := __sum + {data.id.name}[__n][__c_index][__h][__w];
                  }}
                }}
                let __divisor: {data_type} = fp_pow_full((({bias} as {data_type}) + (({alpha} as {data_type}) * __sum)), ({beta} as {data_type}));
                {res.id.name}[__n][__c][__h][__w] := {data.id.name}[__n][__c][__h][__w] / __divisor;
              }}
            }}
          }}
        }}
        """,
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
    "concatenate": concatenate,
    "avg_pool2d": avg_pool2d,
    "clip": clip,
    "global_avg_pool2d": global_avg_pool2d,
    "lrn": lrn,
}

# Mapping from Relay binary calls to
# the respective Dahlia operator.
BinaryOps = {"add": "+", "divide": "/", "multiply": "*", "subtract": "-"}


def emit_components(func_defs: List[DahliaFuncDef], save_mem=True) -> str:
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
        if id == "dense":
            dahlia_definitions.append(dense(func_def, save_mem))
        else:
            apply = broadcast if id in BinaryOps else RelayCallNodes[id]
            dahlia_definitions.append(apply(func_def))

    type = func_defs[0].data_type
    imports = [
        f"""import futil("primitives/math.futil")
        {{
          def exp(x: {type}): {type};
          def fp_pow_full(base: {type}, exp_value: {type}): {type};
          def sqrt(in: {type}): {type};
          def fp_sqrt(in: {type}): {type};
        }}"""
    ]

    exp_components = ""
    if any(f.function_id == "lrn" for f in func_defs):
        # Import `exp` operator for softmax.
        sep = type.find(",")
        width = int(type[type.find("<") + 1: sep])
        int_width = int(type[sep + 1: type.find(">")])
        exp_components = generate_fp_pow_full(
            builder=Builder(),
            degree=8,
            width=width,
            int_width=int_width,
            is_signed="u" not in type,
        )
        exp_components = "\n".join(c.doc() for c in exp_components)
    # note that generate_fp_pow_full will already generate an exp component
    # this is why we can do an elif statement
    elif any(f.function_id == "softmax" for f in func_defs):
        # Import `exp` operator for softmax.
        sep = type.find(",")
        width = int(type[type.find("<") + 1: sep])
        int_width = int(type[sep + 1: type.find(">")])
        exp_components = generate_exp_taylor_series_approximation(
            builder=Builder(),
            degree=8,
            width=width,
            int_width=int_width,
            is_signed="u" not in type,
        )
        exp_components = "\n".join(c.doc() for c in exp_components)

    return dahlia_to_calyx(imports, dahlia_definitions) + exp_components
