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
        self.groups = {}
    def visit_var(self, var):
        self.cells[var.name_hint] = 'prim std_add(32)'
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

        return '<{}>'.format(name)

    def visit_call(self, call):
        # Visit the arguments to the call, emitting their control
        # statements.
        arg_stmts = [self.visit(arg) for arg in call.args]
        #currently assume we only have 2 args to add
        print (f"arg_stmts {arg_stmts}")
        if call.op.name == 'add':
            # Create structure for an adder.
            name_add = 'add{}'.format(len(self.cells))
            name_out = '{}_out'.format(name_add)
            self.cells[name_add] = 'prim std_add(32)'
            self.cells[name_out] = 'prim std_reg(32)'
            g_name = 'group{}'.format(len(self.groups))
            self.groups[g_name] = f"\n{name_out}.in = {name_add}.out;\n{name_out}.write_en = 1'd1;\n{g_name}.done = {name_out}.done;\n{name_add}.left={arg_stmts[0]}.out;\n{name_add}.right={arg_stmts[1]}.out;\n"
            return '\n'.join(arg_stmts + ['<run {}>'.format(g_name)])
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
        wires = mk_block(
                'wires',            
                '\n'.join('{}:{{{}}}'.format(k, v)
                      for k, v in self.groups.items())
                )
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
