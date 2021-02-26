from tvm import relay, ir
from tvm.relay.expr_functor import ExprFunctor
from tvm.relay.function import Function

from collections import defaultdict
from typing import List, Dict

from relay_utils import *
from futil.ast import *
from dahlia_impl import emit_components


class Relay2Futil(ExprFunctor):
    """The main compilation visitor."""

    def __init__(self):
        super(Relay2Futil, self).__init__()
        self.id_dictionary = defaultdict(int)

        # A dictionary of currently visited variable nodes,
        # since some nodes may be visited more than once.
        self.id_to_cell: Dict[str, Cell] = {}

        # For each Relay CallNode, there is an associated
        # Dahlia FuncDef so that it can be lowered from Dahlia
        # to Calyx as a stand-alone component.
        self.func_defs: List[DahliaFuncDef] = []

        # Controls, wires of the main component.
        self.controls = []
        self.wires = []

    def id(self, name):
        """
        Provides a unique identification for a given name.
        If 'a' is seen twice, it will produce: 'a', 'a1'.
        """
        id_number = self.id_dictionary[name]
        self.id_dictionary[name] += 1
        return f'{name}{"" if id_number == 0 else id_number}'

    def visit_var(self, var) -> Cell:
        """Visits a Relay variable and returns the
        corresponding Calyx memory.
        """
        id = self.id(var.name_hint)
        cell = get_memory(id, var.type_annotation)

        # Some variables may be visited more than once,
        # e.g. if it both a destination and used as
        # an argument.
        if id not in self.id_to_cell:
            self.id_to_cell[id] = cell

        return cell

    def visit_let(self, let):
        """Visits a `let` statement in the following manner:
        1. Visit the `value`.
        2. Visit the `var`, or destination.
        3. Return the `body`.
        """
        value = self.visit(let.value)
        dest = self.visit(let.var)

        # TODO(cgyurgyik): Support other let values.
        # We'll need to support constants for `VGG Net`.
        if not isinstance(value, tvm.relay.Call):
            assert 0, f'{value} is unsupported.'

        # Function names may have a namespace
        # prepended, e.g. `nn.bias_add`. We want to
        # replace the periods, to get `nn_bias_add`.
        func_name = (value.op.name).replace('.', '_')

        comp_id = self.id(func_name)
        comp_decl = CompVar(f'{comp_id}_')
        self.id_to_cell[comp_id] = Cell(
            comp_decl,
            CompInst(comp_id, [])
        )

        self.controls.append(
            # Append Invoke control to the `main` component.
            emit_invoke_control(comp_decl, dest, value.args)
        )

        self.func_defs.append(
            DahliaFuncDef(
                function_id=func_name,
                dest=dest,
                args=value.args,
                attributes=value.attrs,
                data_type=get_dahlia_data_type(let.var.type_annotation)
            )
        )

        return self.visit(let.body)

    def visit_constant(self, const) -> Cell:
        assert 0, f'visit_constant is not supported yet: {const}'
        # type, shape = const.data.dtype, const.data.shape

    def visit_call(self, call):
        """The Relay call consists of 3 main pieces:
        call.op, call.args, and call.attrs. The
        latter two are used within call.op.

        call.op is mapped to a corresponding Dahlia function,
        and subsequently lowered to Calyx as a component to
        be invoked.
        """
        # Visit the call arguments.
        call.args = [self.visit(a) for a in call.args]
        return call

    def visit_function(self, function):
        """Visits the function. Returns the `main`
        component, as well as a list of Dahlia
        function definitions."""
        for p in function.params:
            self.visit(p)

        self.visit(function.body)

        return (
            Component(
                name='main',
                inputs=[],
                outputs=[],
                structs=self.wires + list(self.id_to_cell.values()),
                controls=SeqComp(self.controls)
            ),
            self.func_defs
        )


def relay_transforms(expr: Function) -> Function:
    """https://tvm.apache.org/docs/api/python/relay/transform.html"""
    transforms = tvm.transform.Sequential([
        relay.transform.InferType(),
    ])
    mod = ir.IRModule.from_expr(expr)
    mod = transforms(mod)
    return mod['main']


def emit_futil(program) -> str:
    """Lowers a Relay function to a Calyx program."""
    relay_program = relay_transforms(program)
    visitor = Relay2Futil()
    main, func_defs = visitor.visit(relay_program)
    return '\n'.join((
        Program(
            imports=[
                Import("primitives/std.lib"),
                Import("primitives/bitnum/math.futil")
            ],
            components=[main]
        ).doc(),
        emit_components(func_defs)
    ))


if __name__ == '__main__':
    import sys

    relay_function = relay.fromtext(sys.stdin.read())
    print(
        emit_futil(relay_function)
    )
