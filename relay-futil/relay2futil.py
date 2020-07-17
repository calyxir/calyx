from tvm.relay.expr_functor import ExprFunctor
import textwrap

PREAMBLE = """
import "primitives/std.lib";
"""


def mk_block(decl, contents, indent=2):
    """Format a block like this:

        decl {
          contents
        }

    where `decl` is one line but contents can be multiple lines.
    """
    return decl + ' {\n' + textwrap.indent(contents, indent * ' ') + '\n}'


class Relay2Futil(ExprFunctor):
    """Our main compilation visitor.
    """
    def __init__(self):
        super(Relay2Futil, self).__init__()

        # The visit builds up pieces of the FuTIL component
        # representation mutably: it adds to these data structures. This
        # *really* means that we currently only support one component:
        # we would need to do something fancier if we wanted to support
        # different components with different sets of cells and wires.
        self.cells = {}  # Maps names to definitions.

    def visit_var(self, var):
        return "<variable {}>".format(var.name_hint)

    def visit_constant(self, const):
        # We only handle scalar integers for now.
        assert const.data.dtype == 'int32', \
            "unsupported constant: {}".format(const.data.dtype)
        assert const.data.shape == (), \
            "unsupported array shape: {}".format(const.data.shape)
        # Is this the "right" way to unwrap?
        value = int(const.data.asnumpy())

        # Create structure for the constant.
        name = 'const{}'.format(len(self.cells))
        self.cells[name] = 'prim std_const({}, {})'.format(
            32,     # Bit width.
            value,  # The constant integer value.
        )

        return '<read {}>'.format(name)

    def visit_call(self, call):
        # Visit the arguments to the call, emitting their control
        # statements.
        arg_stmts = [self.visit(arg) for arg in call.args]

        if call.op.name == 'add':
            # Create structure for an adder.
            name = 'add{}'.format(len(self.cells))
            self.cells[name] = 'prim std_add(32)'

            return '\n'.join(arg_stmts + ['<run {}>'.format(name)])
        else:
            assert False, 'unsupported op: {}'.format(call.op.name)

    def visit_function(self, func):
        body = self.visit(func.body)

        # Construct a FuTIL component. For now, the component is
        # *always* called `main`. Someday, we should actually support
        # multiple functions as multiple components.
        cells = mk_block(
            'cells',
            '\n'.join('{} = {};'.format(k, v)
                      for k, v in self.cells.items()),
        )
        wires = mk_block('wires', '')
        control = mk_block('control', body)
        component = mk_block(
            'component main() -> ()',
            '\n'.join([cells, wires, control])
        )

        return component


def compile(program) -> str:
    """Translate a Relay function to a FuTIL program (as a string).
    """
    visitor = Relay2Futil()
    src = visitor.visit(program)
    return "{}\n{}".format(PREAMBLE.strip(), src)
