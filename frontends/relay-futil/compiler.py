import tvm
from tvm import relay, ir
from tvm.relay.expr_functor import ExprFunctor
from tvm.relay.function import Function
from collections import defaultdict

from pretty_print import *
from utilities import *
from futil_ast import *
from dahlia_functions import *


class Relay2Futil(ExprFunctor):
    """The main compilation visitor."""

    def __init__(self):
        super(Relay2Futil, self).__init__()
        self.id_dictionary = defaultdict(int)
        self.relay_id_dictionary = defaultdict(int)
        self.main = FComponent(name="main")

    def id(self, name):
        """
        Provides a unique identification for a given name.
        For example, if 'a' is seen three times, it will produce: 'a0', 'a1', 'a2'.
        """
        id_number = str(self.id_dictionary[name])
        self.id_dictionary[name] += 1
        return ''.join((name, id_number))

    def relay_id(self, name):
        """
        Relay does not explicitly differentiate a variable name if it is used twice. For example,
        %x  = foo(%y);
        %x1 = bar(%x); // Here, at this level, the name_hint associated with `x1` is still 'x'.

        To avoid this, we provide Relay with its own identification dictionary. If 'x' is seen
        three times, it will produce: 'x', 'x1', x2'.
        """
        id_number = self.relay_id_dictionary[name]
        self.relay_id_dictionary[name] += 1
        if id_number == 0: return name
        return ''.join((name, str(id_number)))

    def dahlia_name(self, name, type):
        """
        Dahlia uses the following naming scheme for arbitrary variables `X`, `Y`:
        Memory1D: `X0`, `Y0`
        Memory2D: `X0_0`, `Y0_0`
        Memory3D: `X0_0_0`, `Y0_0_0`
        """
        assert type in DahliaNameExtension, f'{name} with {type} is not supported yet.'
        return ''.join((name, DahliaNameExtension[type]))

    def visit_var(self, var) -> FCell:
        name = self.relay_id(var.name_hint)
        if name in self.main.cells: return cell
        data, type, data_type = get_memory_parameters(var.type_annotation)
        return FCell(dahlia_name=self.dahlia_name(name, type),
                     primitive=FPrimitive(name=name, data=data, data_type=data_type, type=type))

    def visit_let(self, let):
        values, output = self.visit(let.value), self.visit(let.var)
        if isinstance(values, list):
            for value in values:
                if value.is_relay_function(): value.relay_function.output = output
        return [self.visit(let.body), values]

    def visit_constant(self, const) -> FCell:
        # Note: We're currently treating constants defined in a `let` statement in Relay IR as 1D Memory.
        # type, shape = const.data.dtype, const.data.shape
        pass

    def visit_call(self, call) -> List[FCell]:
        attributes = call.attrs
        cells, args = [], []
        for arg in call.args:
            argument = self.visit(arg)
            cells.append(argument)
            args.append(argument)
        # We are representing all function calls in Relay IR at the Dahlia level, which will then be lowered to FuTIL.
        # Note, the Relay function's output is not defined until the `let` statement is visited.
        function, name, op = GetRelayFunctionCall(call.op.name)
        relay_function_call = RelayFunctionCall(component_name=self.relay_id(name), name=self.id(name), op=op,
                                                inputs=args, attributes=call.attrs, lowering_function=function)
        cells.append(FCell(relay_function=relay_function_call))
        return cells

    def visit_function(self, function):
        body = self.visit(function.body)
        for cell in flatten(body): self.main.add_cell(cell)
        build_main_controls(self.main)
        return pp_lowered_relay_function(self.main)


def relay_transforms(expr: Function) -> Function:
    """https://tvm.apache.org/docs/api/python/relay/transform.html"""
    transform = tvm.transform.Sequential([
        relay.transform.SimplifyExpr(),
        relay.transform.SimplifyInference(),
        relay.transform.InferType()
    ])
    mod = ir.IRModule.from_expr(expr)
    mod['main'] = expr
    mod = transform(mod)
    return mod['main']


def lower_to_futil(program) -> str:
    """Translate a Relay function to a FuTIL program (as a string)."""
    program = relay_transforms(program)
    visitor = Relay2Futil()

    PREAMBLE = """import "primitives/std.lib";\n"""
    MAIN = visitor.visit(program)
    return '\n'.join((PREAMBLE, MAIN))


if __name__ == '__main__':
    import sys

    relay_function = relay.fromtext(sys.stdin.read())
    print(lower_to_futil(relay_function))
