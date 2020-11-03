from futil_ast import *
from itertools import chain


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
    begin = 3 if t[0:3] == 'int' else 5 # 'float'
    return int(t[begin:len(t)])


def extract_function_arguments(args):
    '''
    Extracts the arguments from a function as port definitions.
    '''
    inputs = []
    outputs = []
    for arg in args:
        name = arg.name_hint
        bitwidth = get_bitwidth(arg.type_annotation)
        out_port = f'{name}_out'
        inputs.append(FPortDef(name=out_port, bitwidth=bitwidth))
    inputs.append(FPortDef(name="in_done", bitwidth = 1))

    write_data_port = f'in_write_data'
    write_enable_port = f'in_write_en'
    addr0_port = f'in_addr0'

    outputs.append(FPortDef(name=write_data_port, bitwidth=bitwidth))
    # TODO(cgyurgyik): Let's instead add a begin and end index. If begin == end, we can assume its 0D.
    outputs.append(FPortDef(name=write_enable_port, bitwidth=1))
    outputs.append(FPortDef(name=addr0_port, bitwidth=1))  # FIXME: Hardcoded for 0D tensors.
    return inputs, outputs


def build_main_body(c: FComponent):
    '''
    Builds the main function that will take the last function and run it.
    '''
    for cell in reversed(c.cells):
        if cell.is_declaration():
            bitwidth = cell.declaration.component.signature.outputs[0].bitwidth
            inputs = cell.declaration.component.signature.inputs
            outputs = cell.declaration.component.signature.outputs
            function_name = cell.declaration.name
            break

    index = 0
    cst = FCell(primitive=FPrimitive(name=f'c{index}', data=[1, index], type=PrimitiveType.Constant))
    c.add_cell(cst)
    ret = FCell(primitive=FPrimitive(name=f'{c.name}_ret', data=[32, 1, 1], type=PrimitiveType.Memory1D))
    c.add_cell(ret)

    input_arguments = []
    for i in range(0, len(inputs) - 1):
        input_name = (inputs[i].name).split('_')[0]
        input_arguments.append(input_name)
        c.add_cell(FCell(primitive=FPrimitive(name=input_name, data=[bitwidth, 1, 1], type=PrimitiveType.Memory1D)))

    group_name = f'run_{function_name}'
    write_data_port = outputs[0].name
    write_enable_port = outputs[1].name
    addr0_port = outputs[2].name

    wires = []
    for i in range(0, len(input_arguments)):
        # Build connections for input arguments.
        wires.append(FWire(f'{function_name}.{inputs[i].name}', f'{input_arguments[i]}.read_data'))
        wires.append(FWire(f'{input_arguments[i]}.addr0', f'{function_name}.{addr0_port}'))

    wires.append(FWire(f'{c.name}_ret.addr0', f'{function_name}.{addr0_port}'))
    wires.append(FWire(f'{c.name}_ret.write_data', f'{function_name}.{write_data_port}'))
    wires.append(FWire(f'{c.name}_ret.write_en', f'{function_name}.{write_enable_port}'))
    wires.append(FWire(f'{function_name}.in_done', f'{ret.primitive.name}.done'))
    wires.append(FWire(f'{function_name}.go', "1'd1"))
    wires.append(FWire(f'{group_name}[done]', f'{function_name}.done ? ' + "1'd1"))

    c.wires = [FConnection(group=FGroup(name=group_name, wires=wires, attributes=[]))]
    connections = list(filter(lambda w: w.is_group(), c.wires))
    c.controls = [Seq(stmts=list(map(lambda w: w.group.name, connections)))]


def build_function_body(c: FComponent):
    '''
    Builds the body of the relay function. This is done by building function declarations,
    and connecting them with wires.
    '''
    declarations = []
    for cell in reversed(c.cells):
        if cell.is_declaration():
            declarations.append(cell.declaration)

    for declaration in declarations:
        intermediary_output = declaration.intermediary_output
        c.add_cell(declaration.intermediary_output)
        bitwidth = declaration.component.signature.outputs[0].bitwidth
        inputs = declaration.component.signature.inputs
        outputs = declaration.component.signature.outputs
        function_name = declaration.name
        group_name = f'run_{function_name}'
        write_data_port = outputs[0].name
        write_enable_port = outputs[1].name
        addr0_port = outputs[2].name

        wires = get_input_wires(c, declaration)
        wires.append(FWire(f'{intermediary_output.primitive.name}.write_data', f'{function_name}.{write_data_port}'))
        wires.append(FWire(f'{intermediary_output.primitive.name}.write_en', f'{function_name}.{write_enable_port}'))
        wires.append(FWire(f'{intermediary_output.primitive.name}.addr0', f'{function_name}.{addr0_port}'))
        wires.append(FWire(f'{function_name}.{inputs[-1].name}', f'{intermediary_output.primitive.name}.done'))
        wires.append(FWire(f'{function_name}.go', "1'd1"))
        wires.append(FWire(f'{group_name}[done]', f'{function_name}.done ? ' + "1'd1"))
        c.wires.append(FConnection(group=FGroup(name=group_name, wires=wires, attributes=[])))

    last = declarations[len(declarations) - 1].intermediary_output
    build_return_connections(c, last)

    # Ensures that only group names make it into the Controls of a component.
    connections = list(filter(lambda w: w.is_group(), c.wires))
    c.controls = [Seq(stmts=list(map(lambda w: w.group.name, connections)))]
    return c


def get_input_wires(comp: FComponent, decl: FDeclaration):
    '''
    Produces the appropriate input wires for a declaration 'decl' within component 'c'.
    This is necessary to avoid re-creating cells for intermediary inputs that
    already exist. For example,

    fn(%a, %b) {
      let %c = multiply(%a, %b); // %a, %b already exist.
      let %d = add(%a, %c);      // %c is an intermediary.
    }
    '''
    function_name = decl.name
    decl_inputs = decl.component.signature.inputs
    intermediary_inputs = flatten(decl.intermediary_inputs)

    finalized_inputs = []
    # Determines whether an input is either an actual input of a previous function or an intermediary input.
    # TODO(cgyurgyik): Clean this up once finalized, and use appropriate data structures.
    for input in intermediary_inputs:
        found = False
        for cell in comp.cells:
            if not cell.is_primitive() or cell.primitive.name != input.primitive.name: continue
            found = True
            finalized_inputs.append(f'{input.primitive.name}.read_data')
            break
        if not found:
            finalized_inputs.append(f'{input.primitive.name}_out')

    wires = []
    for i in range(0, len(decl_inputs) - 1):
        # Build connections for input arguments.
        wires.append(FWire(f'{function_name}.{decl_inputs[i].name}', f'{finalized_inputs[i]}'))
    return wires


def build_return_connections(comp: FComponent, intermediary_output: FCell):
    '''
    Given a component `comp` and the final intermediary output `intermediary_output`, Creates a group to save the value in main.
    Example:
        Relay Function:
        fn (%a, %b) {
          let %c = add(%a, %b);
          %c
        }
        This will create the group (and corresponding wires) to connect `c` to the return value in `main`.
    '''
    inputs = comp.signature.inputs
    outputs = comp.signature.outputs
    intermediary_output_name = intermediary_output.primitive.name

    index = primitive = FPrimitive(name="c0", data=[1, 0], type=PrimitiveType.Constant)
    comp.add_cell(FCell(primitive=index))

    group_name = "save_return_value"
    wires = []
    wires.append(FWire(f'{intermediary_output_name}.addr0', f'{index.name}.out'))
    wires.append(FWire(f'in_addr0', f'{index.name}.out'))
    wires.append(FWire(f'in_write_en', "1'd1"))
    wires.append(FWire(f'in_write_data', f'{intermediary_output_name}.read_data'))
    wires.append(FWire(f'{group_name}[done]', f'{inputs[-1].name} ? ' + "1'd1"))
    comp.wires.append((FConnection(group=FGroup(name=group_name, wires=wires, attributes=[]))))


def build_tensor_0D_binary_op(call, args, op_name: str):
    '''
    Builds the component for a 0D tensor (scalar) binary operation.
    '''
    comp: FComponent = FComponent(name=op_name, cells=[], wires=[],
                               signature=FSignature(inputs=[], outputs=[]))
    inputs, outputs = extract_function_arguments(call.args)
    comp.signature.inputs = inputs
    comp.signature.outputs = outputs

    op = op_name
    assert inputs[0].bitwidth == inputs[1].bitwidth, \
        f'Port definitions have different bitwidths for BinOp: {inputs[0].bitwidth}, {inputs[1].bitwidth}'

    cst = FCell(primitive=FPrimitive(name="c0", data=[inputs[-1].bitwidth, 0], type=PrimitiveType.Constant))
    adder = FCell(primitive=FPrimitive(name=op, data=[inputs[0].bitwidth, op_name], type=PrimitiveType.BinOp))
    comp.add_cell(adder)
    comp.add_cell(cst)

    write_data_port = outputs[0].name
    write_en_port = outputs[1].name
    addr0_port = outputs[2].name

    group_name = f'process_{op_name}'
    wires = []
    wires.append(FWire(addr0_port, f'{cst.primitive.name}.out'))
    wires.append(FWire(f'{op}.left', inputs[0].name))
    wires.append(FWire(f'{op}.right', inputs[1].name))
    wires.append(FWire(write_en_port, "1'd1"))
    wires.append(FWire(write_data_port, f'{op}.out'))

    wires.append(FWire(f'{group_name}[done]', f'{inputs[-1].name} ? ' + "1'd1"))

    connections = [FConnection(group=FGroup(name=group_name, wires=wires, attributes=[]))]
    comp.wires = connections
    comp.controls = [Seq(stmts=list(map(lambda w: w.group.name, connections)))]
    return FCell(declaration=FDeclaration(name=op_name + "_fn", component=comp, intermediary_inputs=args))
