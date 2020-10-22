from futil_ast import *


def mk_block(decl, contents, indent=2):
    """Format a block like this:
        decl {
          contents
        }
    where `decl` is one line but contents can be multiple lines.
    """
    return decl + ' {\n' + textwrap.indent(contents, indent * ' ') + '\n}'


def pp_component_signature(component: FComponent):
    inputs = []
    if component.signature == None:
        return "", ""

    for input in component.signature.inputs:
        inputs.append(f'{input.name}: {input.bitwidth}')

    outputs = []
    for output in component.signature.outputs:
        outputs.append(f'{output.name}: {output.bitwidth}')

    return ', '.join(inputs), ', '.join(outputs)


def pp_wire(wire: FWire):
    return f'{wire.src} = {wire.dst};'


def pp_connections(component: FConnection):
    connections = []
    for connection in component.wires:
        if connection.is_wire():
            connections.append(pp_wire(connection.wire))
        elif connection.is_group():
            wires = []
            for wire in connection.group.wires:
                wires.append(pp_wire(wire))
            connections.append(mk_block(f'group {connection.group.name}', '\n'.join(wires)))
    return connections


def pp_control(component: FComponent):
    ctrls = []
    for control in component.controls:
        groups = []
        for group_name in control.stmts:
            groups.append(f'{group_name};')
        ctrls.append(mk_block(control.name, '\n'.join(groups)))
    return ctrls


def pp_component(component: FComponent):
    subcomponents = []
    for cell in component.cells:
        if cell == None:
            continue
        subcomponents.append(pp_cell(cell))
    cells = mk_block("cells", '\n'.join(subcomponents))

    inputs, outputs = pp_component_signature(component)

    wires = mk_block("wires", '\n'.join(pp_connections(component)))

    controls = "" if component.controls == None else '\n'.join(pp_control(component))
    control = mk_block("control", controls)

    return mk_block(f'component {component.name} ({inputs}) -> ({outputs})', '\n'.join([cells, wires, control]))


def pp_cell(cell: FCell):
    if cell.is_primitive():
        data = cell.primitive.data
        bitwidth = str(data[0])
        if cell.primitive.type == PrimitiveType.Register:
            return f'{cell.primitive.name} = prim std_reg({bitwidth});'
        elif cell.primitive.type == PrimitiveType.Constant:
            value = str(data[1])
            return f'{cell.primitive.name} = prim std_const({bitwidth}, {value});'
        elif cell.primitive.type == PrimitiveType.Memory1D:
            bitwidth = str(data[0])
            size = str(data[1])
            index_size = str(data[2])
            return f'{cell.primitive.name} = prim std_mem_d1({bitwidth}, {size}, {index_size});'
        else:
            assert False, f'FCell pretty print unimplemented for {cell} with name {cell.primitive.name}'
    elif cell.is_declaration():
        return f'{cell.declaration.name} = {cell.declaration.component.name};'
