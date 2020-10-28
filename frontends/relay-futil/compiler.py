from tvm import relay, ir
from tvm.relay.expr_functor import ExprFunctor
from tvm.relay.function import Function
import textwrap
from collections import namedtuple, defaultdict
import math

from pretty_print import *
from utilities import *
from futil_ast import *

# Map standard Relay call to respective hardware name in FuTIL.
BuiltInBinaryCalls = {'add': 'add', 'equal': 'eq', 'multiply': 'mult', 'subtract': 'sub'}

EmitResult = namedtuple('EmitResult', ['cells', 'groups'])


class Relay2Futil(ExprFunctor):
    """The main compilation visitor."""

    def id(self, name):
        """
        Provides unique identification for a given name.
        """
        id_number = self.id_dictionary[name]
        self.id_dictionary[name] += 1
        return name + str(id_number)

    def __init__(self):
        super(Relay2Futil, self).__init__()
        self.id_dictionary = defaultdict(int)
        self.main = FComponent(name="main", cells=[], wires=[])

    def visit_var(self, var):
        name = var.name_hint
        type = str(var.type_annotation)
        data = [get_bitwidth(type), 1, 1]  # [width, size, index_size]
        return [FCell(primitive=FPrimitive(name=name, data=data, type=PrimitiveType.Memory1D))]

    def visit_let(self, let):
        variable = self.visit(let.var)[0]
        body = self.visit(let.body)
        values = self.visit(let.value)

        for value in values:
            if not value.is_declaration(): continue
            value.declaration.intermediary_output = FCell(
                primitive=FPrimitive(name=variable.primitive.name, data=variable.primitive.data,
                                     type=PrimitiveType.Memory1D))
        return [body, values]

    def visit_constant(self, const):
        type = const.data.dtype
        shape = const.data.shape
        data = [get_bitwidth(type), int(const.data.asnumpy())]  # [width, value]
        name = self.id("const")
        return [FCell(primitive=FPrimitive(name=name, data=data, type=PrimitiveType.Constant))]

    def visit_call(self, call):
        assert call.op.name in BuiltInBinaryCalls, f'{call.op.name} not supported.'
        op = BuiltInBinaryCalls[call.op.name]

        args = []
        for arg in call.args: args.append(self.visit(arg))
        return [build_tensor_0D_binary_op(call, args, op)]

    def visit_function(self, function):
        fn: FComponent = FComponent(name=self.id("function"), cells=[], wires=[],
                                    signature=FSignature(inputs=[], outputs=[]))
        fn.signature.inputs, fn.signature.outputs = extract_function_arguments(function.params)
        body = self.visit(function.body)

        components = [fn]
        for cell in flatten(body):
            if cell.is_declaration():
                fn.add_cell(cell)
                components.append(cell.declaration.component)
            elif cell.primitive.type == PrimitiveType.Constant:
                # Include constants, but not function arguments.
                fn.add_cell(cell)

        build_function_body(fn)  # Groups, wires, connections.

        # Add declaration to main.
        self.main.add_cell(FCell(declaration=FDeclaration(name=self.id("fn"), component=fn)))

        return '\n'.join(pp_component(c) for c in reversed(components))


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


def compile(program) -> str:
    """Translate a Relay function to a FuTIL program (as a string)."""
    program = infer_type(program)
    visitor = Relay2Futil()
    src = visitor.visit(program)

    build_main_body(visitor.main)
    PREAMBLE = """import "primitives/std.lib";"""
    NEWL = "\n\n"
    return f'{PREAMBLE}{NEWL}{src}{NEWL}{pp_component(visitor.main)}'


if __name__ == '__main__':
    import sys

    relay_func = relay.fromtext(sys.stdin.read())
    print(compile(relay_func))
