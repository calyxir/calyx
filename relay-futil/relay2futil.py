from tvm.relay.expr_functor import ExprFunctor
import textwrap
from collections import namedtuple

PREAMBLE = """
import "primitives/std.lib";
"""

EmitResult = namedtuple('EmitResult',
                        ['value', 'done', 'cells', 'wires'])


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
        self.next_id = 0

    def fresh_id(self):
        the_id = self.next_id
        self.next_id += 1
        return the_id

    def visit_var(self, var):
        name = var.name_hint
        return EmitResult(
            f'{name}.out',  # Assuming variables are in registers.
            f'{name}[done]',
            [],
            [],
        )

    def visit_constant(self, const):
        # We only handle scalar integers for now.
        assert const.data.dtype == 'int32', \
            "unsupported constant: {}".format(const.data.dtype)
        assert const.data.shape == (), \
            "unsupported array shape: {}".format(const.data.shape)
        # Is this the "right" way to unwrap?
        value = int(const.data.asnumpy())

        # Create structure for the constant.
        name = 'const{}'.format(self.fresh_id())
        cell = '{} = prim std_const({}, {});'.format(
            name,
            32,     # Bit width.
            value,  # The constant integer value.
        )

        return EmitResult(
            f'{name}.out',
            None,
            [cell],
            [],
        )

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

        # Make registers for the arguments.
        func_cells = []
        for param in func.params:
            # TODO: Check the types of the arguments, just like in the
            # visit_var method above.
            name = param.name_hint
            func_cells.append(f'{name} = prim std_reg(32);')

        # Make a register for the return value.
        func_cells.append('ret = prim std_reg(32);')

        # Create a group for the wires that run this expression.
        group_name = 'group{}'.format(self.fresh_id())
        group_wires = body.wires + [
            f'ret.in = {body.value};',
            f'ret.write_en = {body.done};',
            f'{group_name}[done] = ret[done];',
        ]
        group = mk_block(f'group {group_name}', '\n'.join(group_wires))

        # Construct a FuTIL component. For now, the component is
        # *always* called `main`. Someday, we should actually support
        # multiple functions as multiple components.
        cells = mk_block('cells', '\n'.join(func_cells + body.cells))
        wires = mk_block('wires', group)  # Just one group.
        control = mk_block('control', group_name)  # Invoke the group.
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
