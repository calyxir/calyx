from tvm import relay, ir
from tvm.relay.expr_functor import ExprFunctor
from tvm.relay.function import Function
from collections import defaultdict

from pretty_print import *
from utilities import *
from futil_ast import *
from dahlia_functions import *

# Mapping from Relay binary calls to the respective Dahlia operator.
BuiltInBinaryCalls = {'add': '+', 'multiply': '*', 'subtract': '-'}


class Relay2Futil(ExprFunctor):
    """The main compilation visitor."""

    def __init__(self):
        super(Relay2Futil, self).__init__()
        self.id_dictionary = defaultdict(int)
        self.relay_id_dictionary = defaultdict(int)
        self.dahlia_components = []
        self.main = FComponent(name="main", cells=[], wires=[])

    def id(self, name):
        """
        Provides a unique identification for a given name.
        For example, if 'a' is seen three times, it will produce: 'a0', 'a1', 'a2'.
        """
        id_number = self.id_dictionary[name]
        self.id_dictionary[name] += 1
        return name + str(id_number)

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
        return name + str(id_number)

    def produce_dahlia_name(self, name, type):
        """
        Dahlia uses the following naming scheme for an arbitrary variable 'X':
        Memory1D: 'X0', 'X1', 'X2', ...
        Memory2D: 'X0_0', 'X1_0', 'X2_0', ...
        Memory3D: 'X0_0_0', 'X1_0_0', 'X2_0_0', ...
        """
        dahlia_name = self.id(name)
        if type == PrimitiveType.Memory1D: return dahlia_name
        if type == PrimitiveType.Memory2D: return dahlia_name + "_0"
        if type == PrimitiveType.Memory3D: return dahlia_name + "_0_0"
        assert False, f'{name} with {type} is not supported yet.'

    def get_dahlia_declaration(self, function_name, cells, args):
        """
        Returns the corresponding name, Dahlia function type, and op (if it is a binary op, otherwise None).
        If the function type isn't supported, fails with an assertion.
        """
        input_type = cells[0].primitive.type
        function = name = op = None

        if function_name in BuiltInBinaryCalls:
            op = BuiltInBinaryCalls[function_name]
            if input_type == PrimitiveType.Memory1D:
                function = tensor1d_op
                name = self.relay_id(f'tensor1d_{function_name}')
            if input_type == PrimitiveType.Memory2D:
                function = tensor2d_op
                name = self.relay_id(f'tensor2d_{function_name}')
        if function_name == "nn.batch_flatten":
            if input_type == PrimitiveType.Memory3D:
                function = tensor3d_batch_flatten
                name = self.relay_id(f'{function.__name__}')
        elif function_name == "nn.batch_matmul":
            function = batch_matmul
            name = self.relay_id(f'{function.__name__}')

        assert function != None and name != None, f'{function_name} with type {input_type} is not supported.'
        return DahliaDeclaration(component_name=name, decl_name=self.id(name), op=op, inputs=args, function=function)

    def visit_var(self, var):
        name = self.relay_id(var.name_hint)
        if self.main.contains_primitive(name): return [cell]

        data, type = get_memory_parameters(var.type_annotation)
        dahlia_name = self.produce_dahlia_name(name, type)
        return [FCell(dahlia_name=dahlia_name, primitive=FPrimitive(name=name, data=data, type=type))]

    def visit_let(self, let):
        variable = self.visit(let.var)
        body = self.visit(let.body)
        values = self.visit(let.value)

        output = variable[0]
        for value in flatten(values):
            if not value.is_dahlia_declaration(): continue
            value.dahlia_declaration.output = output
            value.dahlia_declaration.invoke()
        return [body, values]

    def visit_constant(self, const):
        type = const.data.dtype
        shape = const.data.shape
        data = [get_bitwidth(type), int(const.data.asnumpy())]
        name = self.id("const")
        return [FCell(primitive=FPrimitive(name=name, data=data, type=PrimitiveType.Constant))]

    def visit_call(self, call):
        cells = []
        args = []
        for arg in call.args:
            argument = self.visit(arg)
            cells.append(argument)
            args.append(argument)
        cells = flatten(cells)
        cells.append(FCell(dahlia_declaration=self.get_dahlia_declaration(call.op.name, cells, flatten(args))))
        return cells

    def visit_function(self, function):
        body = self.visit(function.body)

        for cell in flatten(body):
            self.main.add_cell(cell)
            if not cell.is_dahlia_declaration(): continue
            self.dahlia_components.append(cell.dahlia_declaration.program)

        build_main(self.main)  # Groups, wires, connections.
        return pp_component(self.main)


def infer_type(expr: Function) -> Function:
    infer_types_pass = relay.transform.InferType()
    fuse_op__pass = relay.transform.FuseOps()
    to_normal_pass = relay.transform.ToANormalForm()
    mod = ir.IRModule()
    mod['main'] = expr
    mod = infer_types_pass(mod)
    ret = mod['main']
    return ret


def compile(program) -> str:
    """Translate a Relay function to a FuTIL program (as a string)."""
    program = infer_type(program)
    visitor = Relay2Futil()

    PREAMBLE = """import "primitives/std.lib";"""
    MAIN = visitor.visit(program)
    DAHLIA_COMPONENTS = '\n'.join(visitor.dahlia_components)
    NEWL = '\n\n'
    return f'{PREAMBLE}{NEWL}{DAHLIA_COMPONENTS}{NEWL}{MAIN}'


if __name__ == '__main__':
    import sys

    relay_func = relay.fromtext(sys.stdin.read())
    print(compile(relay_func))
