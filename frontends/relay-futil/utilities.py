from futil_ast import *
from itertools import chain
import math

# Mapping from the tensor dimensions to the corresponding FuTIL memory type.
NumDimensionsToPrimitive = {1: PrimitiveType.Memory1D, 2: PrimitiveType.Memory2D,
                            3: PrimitiveType.Memory3D, 4: PrimitiveType.Memory4D}


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
    if 'int' in relay_type: return 'ubit'
    if 'float' in relay_type: return 'ufix'
    assert False, f'{relay_type} is not supported.'


def get_bitwidth(relay_type):
    '''
    Gets the bitwidth from a Relay type.
    If the relay_type is floating point of size N, returns a fixed point of size <N, N/2>.
    This lowers to a fixed point cell with `int_width` of size N/2, and a `fract_width` of size N/2.
    '''
    type = str(relay_type)
    length = len(type)
    if 'int' in type: return type[3:length]
    if 'float' in type:
        width = int(type[5:length])
        return f'{width}, {int(width / 2)}'
    assert False, f'{relay_type} is not supported.'


def get_memory_parameters(type):
    '''
    Acquires the memory parameters necessary to create a FuTIL memory primitive.

    A Tensor type in Relay is presented as: `Tensor[(dim1, dim2, ...), type]`.
    For example, `Tensor[(2, 4), int32]` is a 2-dimensional tensor with data type int32.

    We then parse this to determine the corresponding FuTIL and Dahlia types.
    '''
    t = str(type)
    data_type = get_dahlia_data_type(t)
    if t[0:3] == 'int' or t[0:5] == 'float':
        return [get_bitwidth(type), 1, 1], PrimitiveType.Memory1D, data_type
    assert t[0:6] == 'Tensor', f'{type} is not currently supported.'
    string_type = t[t.find(")") + 3:t.find("]")]
    string_dimensions = t[t.find("(") + 1:t.find(")")]

    tensor_dimensions = list(map(int, string_dimensions.split(',')))
    data, num_dimensions = [get_bitwidth(string_type)], len(tensor_dimensions)
    assert num_dimensions in NumDimensionsToPrimitive, f'{num_dimensions} dimensions is not supported.'
    for dimension in tensor_dimensions: data.append(dimension)  # Size.
    for dimension in tensor_dimensions: data.append(int(math.log2(dimension) + 1))  # Index size.
    return data, NumDimensionsToPrimitive[num_dimensions], data_type


def build_main_controls(c: FComponent):
    '''
    Builds the wires and control for the `main` component.
    This is done by creating a group run_* with its respective
    wiring for each Dahlia declaration, and adding it to the
    control.
    '''
    dahlia_declarations = []
    for cell in reversed(c.cells):
        if not cell.is_dahlia_declaration(): continue
        dahlia_declarations.append(cell.dahlia_declaration)

    for declaration in dahlia_declarations:
        inputs = declaration.inputs
        wires = []
        group_name = f'run_{declaration.component_name}'
        for input in flatten(inputs):
            prim = input.primitive
            wires.append(FWire(f'{prim.name}.addr0', f'{declaration.decl_name}.{input.dahlia_name}_addr0'))
            wires.append(
                FWire(f'{declaration.decl_name}.{input.dahlia_name}_read_data', f'{prim.name}.read_data'))
            if prim.type == PrimitiveType.Memory1D: continue
            wires.append(FWire(f'{prim.name}.addr1', f'{declaration.decl_name}.{input.dahlia_name}_addr1'))
            if prim.type == PrimitiveType.Memory2D: continue
            wires.append(FWire(f'{prim.name}.addr2', f'{declaration.decl_name}.{input.dahlia_name}_addr2'))

        output = declaration.output
        wires.append(FWire(f'{output.primitive.name}.addr0', f'{declaration.decl_name}.{output.dahlia_name}_addr0'))
        if output.primitive.type == PrimitiveType.Memory2D or output.primitive.type == PrimitiveType.Memory3D:
            wires.append(FWire(f'{output.primitive.name}.addr1', f'{declaration.decl_name}.{output.dahlia_name}_addr1'))
        if output.primitive.type == PrimitiveType.Memory3D:
            wires.append(FWire(f'{output.primitive.name}.addr2', f'{declaration.decl_name}.{output.dahlia_name}_addr2'))

        wires.append(
            FWire(f'{output.primitive.name}.write_data', f'{declaration.decl_name}.{output.dahlia_name}_write_data'))
        wires.append(
            FWire(f'{output.primitive.name}.write_en', f'{declaration.decl_name}.{output.dahlia_name}_write_en'))
        wires.append(FWire(f'{declaration.decl_name}.{output.dahlia_name}_done', f'{output.primitive.name}.done'))
        wires.append(FWire(f'{declaration.decl_name}.go', "1'd1"))
        wires.append(FWire(f'{group_name}[done]', f"{declaration.decl_name}.done ? 1'd1"))
        c.wires.append(FConnection(group=FGroup(name=group_name, wires=wires, attributes=[])))

    # Ensures that only group names make it into the controls of a component.
    connections = list(filter(lambda w: w.is_group(), c.wires))
    c.controls = [Seq(stmts=list(map(lambda w: w.group.name, connections)))]
