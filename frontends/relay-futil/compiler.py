from tvm import relay, ir
from tvm.relay.expr_functor import ExprFunctor
from tvm.relay.function import Function
import textwrap
from collections import namedtuple, defaultdict
import math

from pretty_print import *
from futil_ast import *

PREAMBLE = """import "primitives/std.lib";"""


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
    Extracts the arguments from a function as port definitions
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


class Relay2Futil(ExprFunctor):
    """The main compilation visitor."""

    def id(self, name):
        """
        Provides unique identification for a given name.
        """
        id_number = self.id_dictionary[name]
        self.id_dictionary[name] += 1
        return str(name + str(id_number))

    def __init__(self):
        super(Relay2Futil, self).__init__()
        self.id_dictionary = defaultdict(int)
        self.main = FComponent(name="main", cells=[], wires=[])

    def visit_var(self, var):
        name = var.name_hint
        type = str(var.type_annotation)
        data = [get_bitwidth(type), 1, 1]  # [width, size, index_size]

        cell = FCell(primitive=FPrimitive(name=name, data=data, type=PrimitiveType.Memory1D))
        self.main.add_cell(cell)
        return cell

    def visit_constant(self, const):
        type = const.data.dtype
        shape = const.data.shape
        data = [get_bitwidth(type), int(const.data.asnumpy())]  # [width, value]
        name = self.id("const")
        return FCell(primitive=FPrimitive(name=name, data=data, type=PrimitiveType.Constant))

    def visit_function(self, function):
        fn_component: FComponent = FComponent(name=self.id("fn"), cells=[], wires=[],
                                              signature=FSignature(inputs=[], outputs=[]))

        # Function Arguments
        inputs, outputs = extract_function_arguments(function.params)
        fn_component.signature.inputs = inputs
        fn_component.signature.outputs = outputs

        body = self.visit(function.body)
        # We want to include constants, but not function arguments.
        if body.primitive.type == PrimitiveType.Constant:
            fn_component.add_cell(body)

        # Return values
        begin_cst = FCell(primitive=FPrimitive(name=self.id("c"), data=[1, 0], type=PrimitiveType.Constant))
        fn_component.add_cell(begin_cst)  # FIXME: Pull data from the input arguments.

        return_type = str(function.ret_type)
        bitwidth = get_bitwidth(return_type)
        ret_cell = FCell(primitive=FPrimitive(name="ret", data=[bitwidth, 1, 1], type=PrimitiveType.Memory1D))
        fn_component.add_cell(ret_cell)

        # Groups
        fn_component.wires = build_return_connections(ret_cell.primitive, begin_cst.primitive, fn_component)

        # Control
        connections = list(filter(lambda w: w.is_group(), fn_component.wires))
        fn_component.controls = [Seq(stmts=list(map(lambda w: w.group.name, connections)))]

        # Add wire to this function.
        self.main.add_cell(FCell(declaration=FDeclaration(name=self.id("function"), component=fn_component)))
        return pp_component(fn_component)


def infer_type(expr: Function) -> Function:
    infer_types_pass = relay.transform.InferType()
    fuse_op__pass = relay.transform.FuseOps()
    to_normal_pass = relay.transform.ToANormalForm()
    mod = ir.IRModule()
    mod['main'] = expr
    # mod = fuse_op__pass(mod)
    mod = infer_types_pass(mod)
    ret = mod['main']
    return ret


def build_main(c: FComponent):
    '''
    Builds the main function that will take the last function and run it.
    '''
    # FIXME(cgyurgyik): This is currently mostly hard-coded.

    for cell in reversed(c.cells):  # Get the bitwidth of the output of the last function declaration.
        if cell.is_declaration():
            bitwidth = cell.declaration.component.signature.outputs[0].bitwidth
            inputs = cell.declaration.component.signature.inputs
            outputs = cell.declaration.component.signature.outputs
            function_name = cell.declaration.name
            break

    ret = FCell(primitive=FPrimitive(name="ret", data=[bitwidth, 1, 1], type=PrimitiveType.Memory1D))
    c.add_cell(ret)

    index = 0
    cst = FCell(primitive=FPrimitive(name=f'c{index}', data=[1, index], type=PrimitiveType.Constant))
    c.add_cell(cst)

    # FIXME: Currently, assuming one input only. For multiple inputs,
    # Need to determine if the input is 1D, 2D, ...
    group_name = f'run_{function_name}'
    var = (inputs[0].name).split('_')[0]
    out_port = inputs[0].name
    done_port = inputs[1].name
    write_data_port = outputs[0].name
    write_enable_port = outputs[1].name
    addr0_port = outputs[2].name

    wire0 = FWire(f'{var}.addr0', f'{cst.primitive.name}.out')
    wire1 = FWire(f'{function_name}.{done_port}', f'{var}.done')
    wire2 = FWire(f'{function_name}.{out_port}', f'{var}.read_data')
    wire3 = FWire(f'{ret.primitive.name}.addr0', f'{function_name}.{addr0_port}')
    wire4 = FWire(f'{ret.primitive.name}.write_data', f'{function_name}.{write_data_port}')
    wire5 = FWire(f'{ret.primitive.name}.write_en', f'{function_name}.{write_enable_port}')
    wire6 = FWire(f'{function_name}.go', "1'd1")
    wire7 = FWire(f'{group_name}[done]', f'{function_name}.done ? ' + "1'd1")

    wires = [wire0, wire1, wire2, wire3, wire4, wire5, wire6, wire7]
    connection_1 = FConnection(group=FGroup(name=group_name, wires=wires, attributes=[]))
    c.wires = [connection_1]

    connections = list(filter(lambda w: w.is_group(), c.wires))
    c.controls = [Seq(stmts=list(map(lambda w: w.group.name, connections)))]


def compile(program) -> str:
    """Translate a Relay function to a FuTIL program (as a string).
    """
    program = infer_type(program)
    visitor = Relay2Futil()
    src = visitor.visit(program)

    build_main(visitor.main)

    return f'{PREAMBLE}\n\n{src}\n\n{pp_component(visitor.main)}'


if __name__ == '__main__':
    import sys

    relay_func = relay.fromtext(sys.stdin.read())
    print(compile(relay_func))
