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


class Relay2Futil(ExprFunctor):
    """The main compilation visitor."""

    def id(self, name):
        """
        Provides unique identification for a given name.
        """
        id_number = self.id_dictionary[name]
        self.id_dictionary[name] += 1
        return str(name + str(id_number))

    def extract_function_arguments(self, args):
        '''
        Extracts the arguments from a function as port definitions
        '''
        port_definitions = []
        for arg in args:
            name = arg.name_hint
            bitwidth = get_bitwidth(arg.type_annotation)
            port_definition = FPortDef(name=name, bitwidth=bitwidth)
            port_definitions.append(port_definition)
        return port_definitions

    def __init__(self):
        super(Relay2Futil, self).__init__()
        self.id_dictionary = defaultdict(int)
        self.main = FComponent(name="main", cells=[], wires=[])

    def visit_var(self, var):
        name = var.name_hint
        type = str(var.type_annotation)

        cell = FCell(primitive=FPrimitive(name=name, data=[get_bitwidth(type)], type=PrimitiveType.Register))

        # The main cell requires a unique identifier when an argument is a function.
        main_cell = FCell(
            primitive=FPrimitive(name=name + "_", data=[get_bitwidth(type)], type=PrimitiveType.Register))
        self.main.add_cell(main_cell)
        return cell

    def visit_constant(self, const):
        type = const.data.dtype
        shape = const.data.shape

        value = int(const.data.asnumpy())
        bitwidth = get_bitwidth(type)

        return FCell(
            primitive=FPrimitive(name=self.id("const"), data=[bitwidth, value], type=PrimitiveType.Constant))

    def visit_function(self, function):
        fn_component: FComponent = FComponent(name=self.id("fn"), cells=[], wires=[],
                                              signature=FSignature(inputs=[], outputs=[]))

        # Function Arguments
        arguments = self.extract_function_arguments(function.params)
        fn_component.signature.inputs = arguments

        body = self.visit(function.body)
        # TODO(cgyurgyik): We want to disclude function arguments.
        # This assumes anything that is not a constant must be a function argument.
        if body.primitive.type == PrimitiveType.Constant:
            fn_component.add_cell(body)

        # Return values
        return_type = str(function.ret_type)
        bitwidth = get_bitwidth(return_type)
        ret_cell = FCell(primitive=FPrimitive(name="ret", data=[bitwidth], type=PrimitiveType.Register))
        fn_component.add_cell(ret_cell)
        fn_component.signature.outputs = [FPortDef(name="out", bitwidth=bitwidth)]

        # Groups
        fn_component.wires = build_return_connections(ret_cell.primitive, fn_component)

        # Control
        connections = list(filter(lambda w: w.is_group(), fn_component.wires))
        fn_component.controls = [Seq(stmts=list(map(lambda w: w.group.name, connections)))]

        # Add wire to this function.
        self.main.add_cell(FCell(declaration=FDeclaration(name=self.id("f"), component=fn_component)))
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
    Builds the main function that will take the last defined relay function and run it.
    '''
    # FIXME(cgyurgyik): This is currently mostly hard-coded.
    for cell in reversed(c.cells):  # Get the bitwidth of the output of the last function declaration.
        if cell.is_declaration():
            bitwidth = cell.declaration.component.signature.outputs[0].bitwidth
            inputs = cell.declaration.component.signature.inputs
            function_name = cell.declaration.name
            break

    # Add a return cell that will store the final output.
    ret_name = "ret"
    ret_cell = FCell(primitive=FPrimitive(name=ret_name, data=[bitwidth], type=PrimitiveType.Register))
    c.add_cell(ret_cell)

    connections = []
    for input in inputs:
        connections.append(FConnection(wire=FWire(f'{function_name}.{input.name}', f'{input.name + "_"}.out')))

    connections.append(FConnection(wire=FWire(f'{function_name}.go', "1'd1")))
    connections.append(FConnection(wire=FWire(f'{ret_name}.write_en', "1'd1")))
    connections.append(FConnection(wire=FWire(f'{ret_name}.in', f'{function_name}.done ? {function_name}.out')))
    c.wires = connections


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
