from tvm import relay, ir
from tvm.relay.expr_functor import ExprFunctor
from tvm.relay.function import Function
import textwrap
from collections import namedtuple, defaultdict
import math

from pretty_print import *
from futil_ast import *

PREAMBLE = """import "primitives/std.lib";"""


class Relay2Futil(ExprFunctor):
    """The main compilation visitor."""

    def __init__(self):
        super(Relay2Futil, self).__init__()
        self.main_component: FComponent = FComponent(cells=[], wires=[])

    def visit_var(self, var):
        name = var.name_hint
        type = str(var.type_annotation)
        assert type[0:3] == "int", "Unsupported variable type: {}.".format(type)

        bitwidth = int(type[3:len(type)])
        # TODO(cgyurgyik): How does one determine what 'type' of
        # variable it is, e.g. std_reg, std_mem_d1, ...
        primitive = FPrimitive(name=name, data=[bitwidth], type=PrimitiveType.Register)
        cell = FCell(primitive=primitive)
        self.main_component.add_wire(cell)
        return cell

    def visit_constant(self, const):
        type = const.data.dtype
        shape = const.data.shape
        assert type[0:3] == 'int', "Unsupported constant type: {}.".format(type)
        assert shape == (), "Unsupported const array shape: {}.".format(shape)

        value = int(const.data.asnumpy())
        bitwidth = type[3:len(type)]

        primitive = FPrimitive(name="const", data=[bitwidth, value], type=PrimitiveType.Constant)
        cell = FCell(primitive=primitive)
        self.main_component.add_wire(cell)
        return cell

    def visit_function(self, function):
        body = self.visit(function.body)
        # function_arguments = function.params
        return_type = function.ret_type

        return pretty_print_component(self.main_component)


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
    """Translate a Relay function to a FuTIL program (as a string).
    """
    program = infer_type(program)
    visitor = Relay2Futil()
    src = visitor.visit(program)
    return "{}\n{}".format(PREAMBLE.strip(), src)


if __name__ == '__main__':
    import sys

    relay_func = relay.fromtext(sys.stdin.read())
    print(compile(relay_func))
