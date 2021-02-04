from typing import List
from futil.ast import *
from dahlia_utils import *


####################################################################################################
################## Dahlia Implementations for Relay Call Nodes #####################################
####################################################################################################

def broadcast(fd: DahliaFuncDef):
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
    op1_dims, op2_dims, res_dims = get_dims(op1.comp), get_dims(op2.comp), get_dims(res.comp)

    # Get memory sizes in reversed order.
    op1_sizes, op2_sizes, res_sizes = [], [], []
    for i in reversed(range(0, op1_dims)): op1_sizes.append(op1.comp.args[i + 1])
    for i in reversed(range(0, op2_dims)): op2_sizes.append(op2.comp.args[i + 1])
    for i in reversed(range(0, res_dims)): res_sizes.append(res.comp.args[i + 1])

    # Gets the last variable name for indexing, since
    # we will compare sizes in the reverse direction.
    index_var = chr(ord(CHARACTER_I) + res_dims - 1)

    # Determine the value address value at each index.
    # This will either be a variable name or `0`.
    index_zero = '[0]'
    op1_indices, op2_indices, res_indices = [], [], []
    for i in range(0, len(res_sizes)):
        current_dimension = f'[{index_var}]'
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
    op1_index = ''.join(reversed(op1_indices))
    op2_index = ''.join(reversed(op2_indices))
    res_index = ''.join(reversed(res_indices))
    loop_body = f'{res.id.name}{res_index} := {op1.id.name}{op1_index} ' \
                f'{BinaryOps[fd.function_id]} {op2.id.name}{op2_index};'

    return emit_dahlia_definition(
        fd,
        emit_dahlia_loop(fd, loop_body, res_dims)
    )


# Mapping from Relay function names to their respective Dahlia lowering.
RelayCallNodes = {
    # 'nn_dense': dense,
    # 'nn_batch_flatten': batch_flatten,
    # 'nn_batch_matmul': batch_matmul,
    # 'nn_bias_add': bias_add,
    # 'nn_relu': relu,
    # 'nn_softmax': softmax,
    # 'nn_max_pool2d': max_pool2d,
    # 'nn_conv2d': conv2d,
    # 'negative': negative,
    # 'expand_dims': expand_dims,
    # 'sqrt': sqrt
}

# Mapping from Relay binary calls to the respective Dahlia operator.
BinaryOps = {
    'add': '+',
    'divide': '/',
    'multiply': '*',
    'subtract': '-'
}


def emit_components(func_defs: List[DahliaFuncDef]) -> str:
    """Returns a string containing all the components
    created from the list of Dahlia function definitions.
    This does not include the import statement.
    """
    dahlia_definitions = []
    for func_def in func_defs:
        id = func_def.function_id
        assert id in RelayCallNodes or id in BinaryOps, f'{id} not supported for lowering.'

        # If the function is a binary operation, use broadcasting.
        # Otherwise, use the associated Relay function.
        apply = broadcast if id in BinaryOps else RelayCallNodes[id]
        dahlia_definitions.append(apply(func_def))
        return dahlia_to_futil('\n'.join(dahlia_definitions))
