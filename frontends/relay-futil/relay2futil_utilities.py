from futil_ast import *

def get_bitwidth(type):
    '''
    Quick and dirty way to get the bitwidth.
    '''
    t = str(type)
    if t[0:3] == 'int':
        return int(t[3:len(t)])
    elif t[0:5] == 'float':
        return int(t[5:len(t)])
    else:
        assert False, f'{t} is not supported.'


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
        done_port = f'{name}_done'
        inputs.append(FPortDef(name=out_port, bitwidth=bitwidth))
        inputs.append(FPortDef(name=done_port, bitwidth=1))

        write_data_port = f'{name}_write_data'
        write_enable_port = f'{name}_write_en'
        addr0_port = f'{name}_addr0'

        outputs.append(FPortDef(name=write_data_port, bitwidth=bitwidth))
        outputs.append(FPortDef(name=write_enable_port, bitwidth=1))
        # TODO(cgyurgyik): Let's instead add a begin and end index.
        outputs.append(FPortDef(name=addr0_port, bitwidth=1))  # FIXME: Hardcoded for scalars.
    return inputs, outputs

def build_return_connections(ret: FPrimitive, index: FPrimitive, comp: FComponent):
    '''
    Given a 'ret' primitive, Creates a group to save the value at `index` for the
    inputs.
    '''
    inputs = comp.signature.inputs
    outputs = comp.signature.outputs
    # Write to return register.

    if len(inputs) > 0:
        input_name = (inputs[0].name).split('_')[0]
    else:
        # If there are no inputs, take the out wire of the last constant.
        for cell in reversed(comp.cells):
            if cell.is_primitive() and cell.primitive.type == PrimitiveType.Constant:
                input_name = f'{cell.primitive.name}.out'
                break

    group_name = "save_return_value"
    wire0 = FWire(f'{ret.name}.addr0', f'{index.name}.out')
    wire1 = FWire(f'{ret.name}.write_en', "1'd1")
    wire2 = FWire(f'{input_name}_addr0', f'{index.name}.out')
    wire3 = FWire(f'{input_name}_write_en', "1'd1")
    wire4 = FWire(f'{ret.name}.write_data', f'{input_name}_out')
    wire5 = FWire(f'{input_name}_write_data', f'{ret.name}.read_data')
    wire6 = FWire(f'{group_name}[done]', f'{ret.name}.done')
    wires = [wire0, wire1, wire2, wire3, wire4, wire5, wire6]

    connection_1 = FConnection(group=FGroup(name=group_name, wires=wires, attributes=[]))
    return [connection_1]
