from __future__ import annotations  # Used for type annotations.
from futil.ast import *
from futil.utils import bits_needed
from dataclasses import dataclass

# Mapping from the tensor dimensions to the corresponding FuTIL memory.
NumDimsToCell = {
    0: Stdlib().register,
    1: Stdlib().mem_d1,
    2: Stdlib().mem_d2,
    3: Stdlib().mem_d3,
    4: Stdlib().mem_d4
}


# TODO(cgyurgyik): Necessary with `Invoke` controls?
def dahlia_name(name, type):
    """Appends the appropriate suffix for Dahlia codegen.
    Dahlia uses the following naming schema
    for an arbitrary variable `X`:
      Memory1D: `X0`
      Memory2D: `X0_0`
      Memory3D: `X0_0_0`
      Memory4D: `X0_0_0_0`
    """
    # DahliaSuffix = {
    #     PrimitiveType.Memory1D: '0',
    #     PrimitiveType.Memory2D: '0_0',
    #     PrimitiveType.Memory3D: '0_0_0',
    #     PrimitiveType.Memory4D: '0_0_0_0'
    # }
    assert type in DahliaSuffix, f'{name} with {type} is not supported.'
    return f'{name}{DahliaSuffix}'


@dataclass
class DahliaFuncDef:
    """Necessary information to compute a Dahlia function definition."""
    component_id: CompVar
    function_id: str
    dest: CompVar
    invoke_ctrl: Invoke
    attributes: tvm.ir.Attrs


def emit_invoke_control(decl: CompVar, dest: Cell, args: List[Cell]) -> List[CompVar]:
    """Returns the input and output connections for Invoke."""
    in_connects=[]
    out_connects=[]

    return Invoke(
        decl,
        in_connects,
        out_connects
    )


def get_dahlia_data_type(relay_type) -> str:
    '''
    Gets the Dahlia data type from the given Relay type.
    NOTE: Currently, Dahlia does not support signed types for arrays.
    '''
    dtype = relay_type.dtype
    if 'int' in dtype: return 'bit'
    if 'float' in dtype: return 'fix'
    assert False, f'{relay_type} is not supported.'


def get_bitwidth(relay_type) -> int:
    '''
    Gets the bitwidth from a Relay type.
    If the relay_type is floating point of width N, returns a fixed point of size <N, N/2>.
    This lowers to a fixed point cell with `int_width` of size N/2, and a `fract_width` of size N/2.
    '''
    dtype = relay_type.dtype
    width = int(''.join(filter(str.isdigit, dtype)))
    if 'int' in dtype:
        return width
    if 'float' in dtype:
        return width, width // 2
    assert 0, f'{relay_type} is not supported.'


def get_memory(name, type) -> Cell:
    dims = type.concrete_shape
    # Bitwidth, along with sizes and index sizes (if it is a Tensor).
    args = [get_bitwidth(type)] + [d for d in dims] + [bits_needed(d) for d in dims]

    num_dims = len(dims)
    return Cell(
        CompVar(name),
        NumDimsToCell[num_dims](*args)
    )
