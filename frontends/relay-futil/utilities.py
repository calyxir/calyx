from futil_ast import *
from itertools import chain
import math


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


def get_bitwidth(type):
    '''
    Quick and dirty way to get the bitwidth.
    '''
    t = str(type)
    assert t[0:3] == 'int' or t[0:5] == 'float', f'{t} is not supported.'
    begin = 3 if t[0:3] == 'int' else 5  # 'float'
    return int(t[begin:len(t)])


def get_memory_parameters(type):
    '''
    Acquires the memory parameters necessary to create a FuTIL memory primitive.
    '''
    t = str(type)
    if t[0:3] == 'int' or t[0:5] == 'float':
        return [get_bitwidth(type), 1, 1], PrimitiveType.Memory1D
    assert t[0:6] == 'Tensor', f'{type} is not currently supported.'

    string_type = t[t.find(")") + 3:t.find("]")]
    string_dimensions = t[t.find("(") + 1:t.find(")")]

    tensor_dimensions = list(map(int, string_dimensions.split(',')))
    data = [get_bitwidth(string_type)]
    for dimension in tensor_dimensions: data.append(dimension)  # Size.
    for dimension in tensor_dimensions: data.append(int(math.log2(dimension) + 1))  # Index size.

    if len(tensor_dimensions) == 2:
        type = PrimitiveType.Memory2D
    elif len(tensor_dimensions) == 3:
        type = PrimitiveType.Memory3D
    return data, type


def build_main(c: FComponent):
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
            if not prim.type == PrimitiveType.Memory2D and not prim.type == PrimitiveType.Memory3D: continue
            wires.append(FWire(f'{prim.name}.addr1', f'{declaration.decl_name}.{input.dahlia_name}_addr1'))
            if not prim.type == PrimitiveType.Memory3D: continue
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

    # Ensures that only group names make it into the Controls of a component.
    connections = list(filter(lambda w: w.is_group(), c.wires))
    c.controls = [Seq(stmts=list(map(lambda w: w.group.name, connections)))]
