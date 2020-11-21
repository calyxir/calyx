import tvm
from tvm import relay, ir
from tvm.relay.expr_functor import ExprFunctor
from tvm.relay.function import Function
from collections import defaultdict

from pretty_print import *
from utilities import *
from futil_ast import *
from dahlia_functions import *

# Mapping from Relay binary calls to the respective Dahlia operator.
BuiltInBinaryOps = {'add': '+', 'divide': '/', 'multiply': '*', 'subtract': '-'}

# Mapping from Tensor dimensions to function type.
BinaryOpTensorDimensions = {PrimitiveType.Memory1D: tensor1d_op, PrimitiveType.Memory2D: tensor2d_op,
                            PrimitiveType.Memory3D: tensor3d_op, PrimitiveType.Memory4D: tensor4d_op}

# Mapping from Relay function names to their respective Dahlia lowering.
RelayFunctionCalls = {'nn.batch_flatten': batch_flatten, 'nn.batch_matmul': batch_matmul,
                      'nn.bias_add': bias_add, 'nn.relu': relu, 'negative': negative, 'expand_dims': expand_dims}


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
        DahliaNameMapping = {PrimitiveType.Memory1D: '', PrimitiveType.Memory2D: '_0',
                             PrimitiveType.Memory3D: '_0_0', PrimitiveType.Memory4D: '_0_0_0'}
        dahlia_name = self.id(name)
        assert type in DahliaNameMapping, f'{name} with {type} is not supported yet.'
        return dahlia_name + DahliaNameMapping[type]

    def get_dahlia_declaration(self, function_name, cells, args, attrs):
        """
        Returns the corresponding name, Dahlia function type, and op (if it is a binary op, otherwise None).
        If the function type isn't supported, fails with an assertion.
        """
        input_type = cells[0].primitive.type
        function = name = op = None
        if function_name in BuiltInBinaryOps:
            op = BuiltInBinaryOps[function_name]
            function, name = BinaryOpTensorDimensions[input_type], function_name
        elif function_name in RelayFunctionCalls:
            function = RelayFunctionCalls[function_name]
            name = function.__name__
        else:
            assert False, f'{function_name} with type {input_type} is not supported.'
        return DahliaDeclaration(component_name=self.relay_id(name), decl_name=self.id(name),
                                 op=op, inputs=args, attributes=attrs, function=function)

    def visit_var(self, var):
        name = self.relay_id(var.name_hint)
        # Do not add duplicate primitives to main.
        if self.main.contains_primitive(name): return cell
        data, type, data_type = get_memory_parameters(var.type_annotation)
        dahlia_name = self.produce_dahlia_name(name, type)
        return FCell(dahlia_name=dahlia_name,
                     primitive=FPrimitive(name=name, data=data, data_type=data_type, type=type))

    def visit_let(self, let):
        values, output = self.visit(let.value), self.visit(let.var)
        for value in values:
            if not value.is_dahlia_declaration(): continue
            value.dahlia_declaration.output = output
            value.dahlia_declaration.invoke()
        body = self.visit(let.body)
        return [body, values]

    def visit_constant(self, const):
        type, shape = const.data.dtype, const.data.shape
        name, data = self.id("const"), [get_bitwidth(type), int(const.data.asnumpy())]
        data_type = get_memory_parameters(type)
        return FCell(primitive=FPrimitive(name=name, data=data, data_type=data_type, type=PrimitiveType.Constant))

    def visit_call(self, call):
        attributes = call.attrs
        cells, args = [], []
        for arg in call.args:
            argument = self.visit(arg)
            cells.append(argument)
            args.append(argument)
        cells.append(FCell(dahlia_declaration=self.get_dahlia_declaration(call.op.name, cells, args, call.attrs)))
        return cells

    def visit_function(self, function):
        body = self.visit(function.body)
        for cell in flatten(body):
            self.main.add_cell(cell)
            if not cell.is_dahlia_declaration(): continue
            self.dahlia_components.append(cell.dahlia_declaration.program)
        build_main_controls(self.main)
        return pp_component(self.main)


def relay_transforms(expr: Function) -> Function:
    """https://tvm.apache.org/docs/api/python/relay/transform.html"""
    transform = tvm.transform.Sequential([
        relay.transform.SimplifyExpr(),
        relay.transform.SimplifyInference(),
        relay.transform.InferType()
    ])
    mod = ir.IRModule()
    mod['main'] = expr
    mod = transform(mod)
    return mod['main']


def compile(program) -> str:
    """Translate a Relay function to a FuTIL program (as a string)."""
    program = relay_transforms(program)
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
