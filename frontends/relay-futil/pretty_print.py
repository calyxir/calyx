from futil_ast import *
import textwrap


def pp_block(decl, contents, indent=2):
    """Format a block like this:
        decl {
          contents
        }
    where `decl` is one line but contents can be multiple lines.
    """
    return ''.join((decl, ' {\n', textwrap.indent(contents, indent * ' '), '\n}'))


def pp_component_signature(component: FComponent):
    inputs = []
    if component.signature == None: return "", ""

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
            connections.append(pp_block(f'group {connection.group.name}', '\n'.join(wires)))
    return connections


def pp_control(component: FComponent):
    ctrls = []
    for control in component.controls:
        groups = []
        for group_name in control.stmts:
            groups.append(f'{group_name};')
        ctrls.append(pp_block(control.name, '\n'.join(groups)))
    return ctrls


def pp_lowered_dahlia_components(component: FComponent):
    relay_functions = []
    for cell in component.cells.values():
        if cell == None or not cell.is_relay_function(): continue
        relay_call = cell.relay_function
        relay_functions.append(relay_call.lowering_function(relay_call))
    return '\n'.join(relay_functions)


def pp_lowered_relay_function(component: FComponent):
    """
    Pretty prints the main program. This consists of the following:
    1. Relay functions lowered from Dahlia -> FuTIL.
    2. The `main` component.

    Example:
    ------------------------------------
    Input
    ```
      fn (%x: int32, %y: int32) { let %z = add(%x, %y); %z }
    ```
    ------------------------------------
    Output
    ```
      component add(...) -> (...) { ... }

      component main() -> () {
        ...
        control { run_add; }
      }
    ```
    """
    relay_function_components = pp_lowered_dahlia_components(component)

    subcomponents = []
    for cell in component.cells.values():
        if cell == None: continue
        subcomponents.append(pp_cell(cell))
    cells = pp_block("cells", '\n'.join(subcomponents))
    inputs, outputs = pp_component_signature(component)
    wires = pp_block("wires", '\n'.join(pp_connections(component)))

    controls = '\n'.join(pp_control(component))
    control = pp_block("control", controls)
    main_component = pp_block(f'component {component.name} ({inputs}) -> ({outputs})',
                              '\n'.join([cells, wires, control]))
    return '\n'.join((relay_function_components, main_component))


def pp_cell(cell: FCell):
    if cell.is_primitive():
        data = cell.primitive.data
        data_type, bitwidth = cell.primitive.data_type, data[0]
        # `fix` / `ufix` will have bitwidth in the form: <TotalWidth, FractWidth>. We only want TotalWidth.
        if data_type == 'ufix' or data_type == 'fix': bitwidth = str(bitwidth).split(',')[0]
        if cell.primitive.type == PrimitiveType.Register:
            return f'{cell.primitive.name} = prim std_reg({bitwidth});'
        if cell.primitive.type == PrimitiveType.Constant:
            value = str(data[1])
            return f'{cell.primitive.name} = prim std_const({bitwidth}, {value});'
        if cell.primitive.type == PrimitiveType.Memory1D:
            size, index_size = str(data[1]), str(data[2])
            return f'{cell.primitive.name} = prim std_mem_d1({bitwidth}, {size}, {index_size});'
        if cell.primitive.type == PrimitiveType.Memory2D:
            size0, size1, index_size0, index_size1 = str(data[1]), str(data[2]), str(data[3]), str(data[4])
            return f'{cell.primitive.name} = prim std_mem_d2({bitwidth}, ' \
                   f'{size0}, {size1}, {index_size0}, {index_size1});'
        if cell.primitive.type == PrimitiveType.Memory3D:
            size0, size1, size2 = str(data[1]), str(data[2]), str(data[3])
            index_size0, index_size1, index_size2 = str(data[4]), str(data[5]), str(data[6])
            return f'{cell.primitive.name} = prim std_mem_d3({bitwidth}, ' \
                   f'{size0}, {size1}, {size2}, {index_size0}, {index_size1}, {index_size2});'
        if cell.primitive.type == PrimitiveType.Memory4D:
            size0, size1, size2, size3 = str(data[1]), str(data[2]), str(data[3]), str(data[4])
            index_size0, index_size1, index_size2, index_size3 = str(data[5]), str(data[6]), str(data[7]), str(data[8])
            return f'{cell.primitive.name} = prim std_mem_d4({bitwidth}, ' \
                   f'{size0}, {size1}, {size2}, {size3}, {index_size0}, {index_size1}, {index_size2}, {index_size3});'
        if cell.primitive.type == PrimitiveType.BinOp:
            op = data[1]
            return f'{cell.primitive.name} = prim std_{op}({bitwidth});'
    if cell.is_relay_function(): return f'{cell.relay_function.name} = {cell.relay_function.component_name};'
    assert False, f'FCell pretty print unimplemented for {cell} with name {cell.primitive.name}'


# Dahlia Pretty Printing.

def next_character(ch, dir=1):
    """
    Returns the next character after 'ch'.
    If `dir` is positive, then will return 'ch' + 1. Otherwise, it will return 'ch' - 1.
    """
    return chr(ord(ch) + 1) if dir > 0 else chr(ord(ch) - 1)


def pp_dahlia_memory_declarations(declaration_list):
    declarations = []
    for declaration in declaration_list:
        string = f'decl {declaration.name}: {declaration.data_type}<{declaration.data[0]}>'
        for i in range(0, declaration.type): string += f'[{declaration.data[i + 1]}]'
        declarations.append(string + ";")
    return '\n'.join(declarations)


def pp_dahlia_loop(data, body):
    """
    Returns an iteration over data with `body` as the work done within the nested loop(s).
    Many tensor functions share the same control flow: (1) Iterate over `data`, and (2) do some work in body.
    For example, if `data` is a 2D primitive of size (M, N) and body == `X;`, then this will return:

    ```
    for (let i: ubit<X> = 0..M) {
      for (let j: ubit<Y> = 0..N) {
        X;
      }
    }
    ```
    """
    variable_name = chr(ord('i'))
    num_dimensions = data.type

    program = []
    SPACING = ''
    for i in range(0, num_dimensions):
        size, index_size = data.data[i + 1], data.data[i + num_dimensions + 1]
        program.append(f'{SPACING}for (let {variable_name}: ubit<{index_size}> = 0..{size}) {{')
        variable_name = next_character(variable_name)
        SPACING += '  '
    program.append(f'{SPACING}{body}')

    for i in range(0, num_dimensions):
        SPACING = SPACING[:-2]
        program.append(f'{SPACING}}}')
    return '\n'.join(program)
