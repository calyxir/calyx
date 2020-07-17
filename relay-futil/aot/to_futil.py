from tvm import relay
from tvm.relay.expr_functor import ExprFunctor
import textwrap

PREAMBLE = """
import "primitives/std.lib";
"""

# For now, components can only be emitted with the name `main`. Someday
# we should actually allow multiple components!
COMPONENT_FMT = """
component main() -> () {}
"""


def mk_block(decl, contents, indent=2):
    """Formats a block like this:

        decl {
          contents
        }

    where `decl` is one line but contents can be multiple lines.
    """
    return decl + ' {\n' + textwrap.indent(contents, indent * ' ') + '\n}'


class ToSource(ExprFunctor):
    def __init__(self):
        super(ToSource, self).__init__()

    def visit_var(self, var):
        return "a variable called {}".format(var.name_hint)

    def visit_function(self, func):
        body = self.visit(func.body)

        # Construct a FuTIL component. For now, the component is
        # *always* called `main`. Someday, we should actually support
        # multiple functions as multiple components.
        cells = mk_block('cells', '')
        wires = mk_block('wires', '')
        control = mk_block('control', body)
        component = mk_block(
            'component main() -> ()',
            '\n'.join([cells, wires, control])
        )

        return component


def to_source(mod, program, gv_map, ctx, name) -> str:
    convert = ToSource()
    src = convert.visit(program)
    return "{}\n{}".format(PREAMBLE.strip(), src)
