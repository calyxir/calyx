from tvm import relay
from tvm.relay.expr_functor import ExprFunctor
from tvm.relay.function import Function
import textwrap
from collections import namedtuple

PREAMBLE = """
import "primitives/std.lib";
"""

EmitResult = namedtuple('EmitResult',
                        ['value', 'done', 'cells', 'wires', 'groups','controls'])


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
            {},
            []
        )

    def visit_constant(self,const):
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
            {},
            []
        )

    def visit_let(self, let):
        # construct a cell for the variable
        name_var = let.var.name_hint
        cell = [f'{name_var} = prim std_reg(32);']
        # start a new group
        group_name = f'group{self.fresh_id()}'
        # visit value expr
        expr_value = self.visit(let.value)
        # add a wire from value.out to name_var
        wires = [f'{name_var}.in = {expr_value.value}'] + expr_value.wires
        # visit the body
        body_value = self.visit(let.body)
        body_value.groups[group_name] = wires
        return EmitResult(
            body_value.value,
            body_value.done,
            body_value.cells + expr_value.cells + cell,
            body_value.wires,
            body_value.groups,
            body_value.controls + [f'{group_name}']
        )

    def visit_call(self, call):
        # Visit the arguments to the call, emitting their control
        # statements.
        arg_stmts = [self.visit(arg) for arg in call.args]
        #currently assume we only have 2 args to add
        structures = [item for arg in arg_stmts for item in arg.cells]
        wires = [item for arg in arg_stmts for item in arg.wires]
        #map standard relay call to hw name in futil
        built_in_calls = {'add':'add', 'subtract':'sub', 'equal':'eq'}
        if built_in_calls[call.op.name]:
            hw_type = built_in_calls[call.op.name]
            # Create structure for an adder.
            cell_name = f'{hw_type}{self.fresh_id()}'
            cell = '{} = prim std_{}({});'.format(
                cell_name,
                hw_type,
                32,     # Bit width.
             )
            structures.append(cell)
            wires.extend([
                    f'{cell_name}.left = {arg_stmts[0].value}',
                    f'{cell_name}.right = {arg_stmts[1].value}' 
                    ])
            return EmitResult(
                    f'{cell_name}.out',
                    None,
                    structures,
                    wires,
                    {},
                    []
                )

    def visit_if(self, if_else):
        # Process conditions
        cond_value = self.visit(if_else.cond)
        cond_name = f'cond{self.fresh_id()}'
        # Process true branch
        true_branch_value = self.visit(if_else.true_branch)
        true_branch_name = f'branch{self.fresh_id()}'
        # Process false branch
        false_branch_value = self.visit(if_else.false_branch)
        false_branch_name = f'branch{self.fresh_id()}'
        # Update groups map
        result_name = f'res{self.fresh_id()}'

        groups = {**true_branch_value.groups, **false_branch_value.groups} 
        groups[cond_name] = cond_value.wires
        groups[true_branch_name] = true_branch_value.wires + [f'{result_name}.in = {true_branch_value.value}']
        groups[false_branch_name] = false_branch_value.wires + [f'{result_name}.in = {false_branch_value.value}']

        structures  = cond_value.cells + true_branch_value.cells +  false_branch_value.cells

        
        true_branch_name 
        return EmitResult(
                f'{result_name}.out',
                None,
                structures,
                [],
                groups,
                [mk_block(f'if {cond_value.value} with {cond_name}',
                    '\n'.join(true_branch_value.controls + 
                    [f'{true_branch_name}'])) + 
                    mk_block('else', '\n'.join(false_branch_value.controls + 
                    [f'{false_branch_name}'])) 
                ]
            )
        

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
        
        groups = mk_block(f'group {group_name}', '\n'.join(group_wires))
        for group in body.groups.keys():
            groups += '\n' + mk_block(f'group {group}', '\n'.join(body.groups[group]))
        # Construct a FuTIL component. For now, the component is
        # *always* called `main`. Someday, we should actually support
        # multiple functions as multiple components.
        cells = mk_block('cells', '\n'.join(func_cells + body.cells))
        wires = mk_block('wires', groups)  # Just one group.
        control = mk_block('control', mk_block('seq', '\n'.join(body.controls + [f'{group_name}'])))  # Invoke the group.
        component = mk_block(
            'component main() -> ()',
            '\n'.join([cells, wires, control])
        )
        
        return component

def optimize(expr: Function) -> Function:
    opts = tvm.transform.Sequential([relay.transform.FuseOps(),relay.transform.ToANormalForm()])
    self.mod['main'] = expr
    self.mod = opts(self.mod)
    ret = self.mod['main']
    return ret

def compile(program) -> str:
    """Translate a Relay function to a FuTIL program (as a string).
    """

    visitor = Relay2Futil()
    src = visitor.visit(program)
    return "{}\n{}".format(PREAMBLE.strip(), src)


if __name__ == '__main__':
    import sys
    relay_func = relay.fromtext(sys.stdin.read())
    print(compile(relay_func))
