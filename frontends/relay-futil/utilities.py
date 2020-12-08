from futil_ast import *
from itertools import chain
import math

# Mapping from the tensor dimensions to the corresponding FuTIL memory type.
NumDimensionsToPrimitive = {1: PrimitiveType.Memory1D, 2: PrimitiveType.Memory2D,
                            3: PrimitiveType.Memory3D, 4: PrimitiveType.Memory4D}

# Mapping between primitive type and associated Dahlia name extension.
# E.g. A 2D memory primitive named `A` will be lowered to `A0_0`.
DahliaNameExtension = {PrimitiveType.Memory1D: '0', PrimitiveType.Memory2D: '0_0',
                       PrimitiveType.Memory3D: '0_0_0', PrimitiveType.Memory4D: '0_0_0_0'}


def flatten(l):
    '''
    Quick and dirty way to flatten a list of lists.
    '''
    new_list = []
    for e in l:
        if type(e) is list and len(e) > 1:
            new_list.extend(flatten(e))
        else:
            while (type(e) is list): e = e[0]
            new_list.append(e)
    return new_list


def get_dahlia_data_type(relay_type):
    '''
    Gets the Dahlia data type from the given Relay type.
    NOTE: Currently, Dahlia does not support signed types for arrays.
    '''
    dtype = relay_type.dtype
    if 'int' in dtype: return 'ubit'
    if 'float' in dtype: return 'ufix'
    assert False, f'{relay_type} is not supported.'


def get_bitwidth(relay_type):
    '''
    Gets the bitwidth from a Relay type.
    If the relay_type is floating point of size N, returns a fixed point of size <N, N/2>.
    This lowers to a fixed point cell with `int_width` of size N/2, and a `fract_width` of size N/2.
    '''
    dtype = relay_type.dtype
    length = len(dtype)
    if 'int' in dtype: return dtype[3:length]
    if 'float' in dtype:
        width = dtype[5:length]
        return f'{width}, {int(width) // 2}'
    assert False, f'{relay_type} is not supported.'


def get_memory_parameters(type):
    '''
    Acquires the memory parameters necessary to create a FuTIL memory primitive.

    A Tensor type in Relay is presented as: `Tensor[(dim1, dim2, ...), type]`.
    For example, `Tensor[(2, 4), int32]` is a 2-dimensional tensor with data type int32.

    We then parse this to determine the corresponding FuTIL and Dahlia types.
    '''
    typ = str(type)
    data_type = get_dahlia_data_type(type)

    if typ[0:3] == 'int' or typ[0:5] == 'float':
        # Currently, we are treating scalar values as 1D Memory primitives.
        return [get_bitwidth(type), 1, 1], PrimitiveType.Memory1D, data_type
    assert typ[0:6] == 'Tensor', f'{type} is not currently supported.'

    tensor_dimensions = type.concrete_shape
    data, num_dimensions = [get_bitwidth(type)], len(tensor_dimensions)
    assert num_dimensions in NumDimensionsToPrimitive, f'{num_dimensions} dimensions is not supported.'
    for dimension in tensor_dimensions: data.append(dimension)  # Size.
    for dimension in tensor_dimensions: data.append(int(math.log2(dimension) + 1))  # Index size.
    return data, NumDimensionsToPrimitive[num_dimensions], data_type


def build_main_controls(c: FComponent):
    '''
    Builds the wires and control for the `main` component. This is done by creating a group `run_*`
    with its respective wiring for each Relay function call, and adding it to the control.
    '''
    for cell in reversed(c.cells.values()):
        if not cell.is_relay_function(): continue
        function = cell.relay_function
        inputs, output = function.inputs, function.output
        wires = []
        group_name = f'run_{function.component_name}'
        for input in flatten(inputs):
            prim = input.primitive
            wires.append(FWire(f'{prim.name}.addr0', f'{function.name}.{input.dahlia_name}_addr0'))
            wires.append(
                FWire(f'{function.name}.{input.dahlia_name}_read_data', f'{prim.name}.read_data'))
            if prim.type == PrimitiveType.Memory1D: continue
            wires.append(FWire(f'{prim.name}.addr1', f'{function.name}.{input.dahlia_name}_addr1'))
            if prim.type == PrimitiveType.Memory2D: continue
            wires.append(FWire(f'{prim.name}.addr2', f'{function.name}.{input.dahlia_name}_addr2'))
            if prim.type == PrimitiveType.Memory3D: continue
            wires.append(FWire(f'{prim.name}.addr3', f'{function.name}.{input.dahlia_name}_addr3'))

        output_type, output_name = output.primitive.type, output.primitive.name
        for i in range(0, 1):
            wires.append(FWire(f'{output_name}.addr0', f'{function.name}.{output.dahlia_name}_addr0'))
            if output_type == PrimitiveType.Memory1D: break
            wires.append(FWire(f'{output_name}.addr1', f'{function.name}.{output.dahlia_name}_addr1'))
            if output_type == PrimitiveType.Memory2D: break
            wires.append(FWire(f'{output_name}.addr2', f'{function.name}.{output.dahlia_name}_addr2'))
            if output_type == PrimitiveType.Memory3D: break
            wires.append(FWire(f'{output_name}.addr3', f'{function.name}.{output.dahlia_name}_addr3'))

        wires.append(FWire(f'{output_name}.write_data', f'{function.name}.{output.dahlia_name}_write_data'))
        wires.append(FWire(f'{output_name}.write_en', f'{function.name}.{output.dahlia_name}_write_en'))
        wires.append(FWire(f'{function.name}.{output.dahlia_name}_done', f'{output_name}.done'))
        wires.append(FWire(f'{function.name}.go', "1'd1"))
        wires.append(FWire(f'{group_name}[done]', f"{function.name}.done ? 1'd1"))
        c.wires.append(FConnection(group=FGroup(name=group_name, wires=wires, attributes=[])))

    # Ensures that only group names make it into the controls of a FuTIL component.
    connections = list(filter(lambda w: w.is_group(), c.wires))
    c.controls = [Seq(stmts=list(map(lambda w: w.group.name, connections)))]
