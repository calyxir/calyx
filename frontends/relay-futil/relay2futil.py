from tvm import relay, ir
from tvm.relay.expr_functor import ExprFunctor
from tvm.relay.function import Function
import textwrap
from collections import namedtuple
import math

from relay2futil_utilities import *

PREAMBLE = """import "primitives/std.lib";"""

# Map standard relay call to respective hardware name in FuTIL.
BuiltInBinaryCalls = {'add': 'add', 'subtract': 'sub', 'equal': 'eq'}

EmitResult = namedtuple('EmitResult',
                        ['value', 'done', 'cells', 'wires', 'groups', 'controls'])


def mk_block(decl, contents, indent=2):
    """Format a block like this:
        decl {
          contents
        }
    where `decl` is one line but contents can be multiple lines.
    """
    return decl + ' {\n' + textwrap.indent(contents, indent * ' ') + '\n}'


class Relay2Futil(ExprFunctor):
    """The main compilation visitor."""

    def __init__(self):
        super(Relay2Futil, self).__init__()
        self.is_ret = True

    def visit_var(self, var):
        name = var.name_hint
        dimension, mem_size, mem_index, bitwidth = ExtractTensorTypes(var.type_annotation)
        value = f'{name}.out' if dimension == 0 else str(name)
        return EmitResult(
            value,  # Assuming variables are in registers.
            f'{name}[done]',
            [],
            [],
            {},
            []
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
        name = 'const{}'.format(id('std_const'))
        cell = '{} = prim std_const({}, {});'.format(
            name,
            32,  # Bit width.
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
        self.is_ret = False
        # construct a cell for the variable
        name_var = let.var.name_hint
        cell = [f'{name_var} = prim std_reg(32);']
        # start a new group
        group_name = f'group{id("group")}'
        # visit value expr
        expr_value = self.visit(let.value)
        # add a wire from value.out to name_var
        wires = [f'{name_var}.in = {expr_value.value};', f"{name_var}.write_en = 1'd1;",
                 f'{group_name}[done] = {name_var}.done;'] + expr_value.wires
        # visit the body
        self.is_ret = True
        body_value = self.visit(let.body)
        body_value.groups[group_name] = wires
        return EmitResult(
            body_value.value,
            body_value.done,
            body_value.cells + expr_value.cells + cell,
            body_value.wires,
            body_value.groups,
            body_value.controls + [f'{group_name};']
        )

    def visit_call(self, call):
        # Visit the arguments to the call, emitting their control statements.
        arg_stmts = [self.visit(arg) for arg in call.args]
        structures = [item for arg in arg_stmts for item in arg.cells]
        wires = [item for arg in arg_stmts for item in arg.wires]
        controls = [item for arg in arg_stmts for item in arg.controls]
        groups = {}
        for arg in arg_stmts:
            groups.update(arg.groups)
        if call.op.name in BuiltInBinaryCalls:
            dimension, memory_size, index_bitwidth, bitwidth = ExtractBinaryArgumentTypes(call.args[0], call.args[1])
            if dimension == 0:  # 0-dimensional tensor, or scalar.
                # Create structure for an adder.
                op = BinaryOp(bitwidth=bitwidth, op=BuiltInBinaryCalls[call.op.name])
                structures.append(op.construct())
                wires.extend([
                    f'{op.name}.left = {arg_stmts[0].value};',
                    f'{op.name}.right = {arg_stmts[1].value};'
                ])
                return EmitResult(
                    f'{op.name}.out',
                    f"1'd1",
                    structures,
                    wires,
                    groups,
                    []
                )
            elif dimension == 1:  # 1-dimensional tensor, or vector.
                op = BinaryOp(bitwidth=bitwidth, op=BuiltInBinaryCalls[call.op.name])
                array_indexing = BinaryOp(bitwidth=index_bitwidth, op="add")
                le_comparator = BinaryOp(bitwidth=index_bitwidth, op="le")
                begin_array = Const(bitwidth=index_bitwidth, value=0)
                end_array = Const(bitwidth=index_bitwidth, value=memory_size - 1)
                increment = Const(name="incr", bitwidth=index_bitwidth, value=1)
                index = Register(name='index', bitwidth=index_bitwidth)
                ret_cell = Tensor1D(bitwidth=bitwidth, memory_size=memory_size, index_bitwidth=index_bitwidth)
                structures.extend([op.construct(), array_indexing.construct(), le_comparator.construct(),
                                   begin_array.construct(), end_array.construct(), increment.construct(),
                                   index.construct(), ret_cell.construct()])

                condition_name = f'cond{id("cond")}'
                groups[condition_name] = [
                    f"{le_comparator.name}.left = {index.name}.out;",
                    f"{le_comparator.name}.right = {end_array.name}.out;",
                    f"{condition_name}[done] = 1'd1;"
                ]
                initialization_name = f'let{id("let")}'
                groups[initialization_name] = [
                    f"{index.name}.in = {begin_array.name}.out;",
                    f"{index.name}.write_en = 1'd1;",
                    f"{initialization_name}[done] = {index.name}.done;"
                ]
                add_body_name = f'body{id("group")}'
                groups[add_body_name] = [
                    f"{ret_cell.name}.addr0 = {index.name}.out;",
                    f"{ret_cell.name}.write_en = 1'd1;",
                    f"{arg_stmts[0].value}.addr0 = {index.name}.out;",
                    f"{arg_stmts[1].value}.addr0 = {index.name}.out;",
                    f"{op.name}.left = 1'd1 ? {arg_stmts[0].value}.read_data;",
                    f"{op.name}.right = 1'd1 ? {arg_stmts[1].value}.read_data;",
                    f"{ret_cell.name}.write_data = {op.name}.out;",
                    f"{add_body_name}[done] = {ret_cell.name}.done ? 1'd1;"
                ]

                update_name = f'update{id("group")}'
                groups[update_name] = [
                    f"{index.name}.write_en = 1'd1;",
                    f"{array_indexing.name}.left = {index.name}.out;",
                    f"{array_indexing.name}.right = {increment.name}.out;",
                    f"{index.name}.in = 1'd1 ? {array_indexing.name}.out;",
                    f"{update_name}[done] = {index.name}.done ? 1'd1;",
                ]

                seq_block = mk_block("seq", "\n".join([f'{add_body_name};', f'{update_name};']))
                mem_control = mk_block(f"while {le_comparator.name}.out with {condition_name}", f'{seq_block}')
                controls.append(f'{initialization_name};')
                controls.append(mem_control)
                return EmitResult(
                    f'{ret_cell.name}',
                    None,
                    structures,
                    wires,
                    groups,
                    controls
                )
            elif dimension == 2:  # 2-dimensional tensor.
                assert(False), "Unimplemented."
            elif dimension == 3:  # 3-dimensional tensor.
                assert(False), "Unimplemented."

    def visit_if(self, if_else):
        # Process conditions
        cond_value = self.visit(if_else.cond)
        cond_name = f'cond{id("cond")}'
        # Process true branch
        true_branch_value = self.visit(if_else.true_branch)
        true_branch_name = f'branch{id("group")}'
        # Process false branch
        false_branch_value = self.visit(if_else.false_branch)
        false_branch_name = f'branch{id("group")}'
        # Update groups map
        result_name = f'res{id("std_reg")}'
        result_cell = f'{result_name} = prim std_reg({32});'

        groups = {**true_branch_value.groups, **false_branch_value.groups}
        groups[cond_name] = cond_value.wires + [f"{cond_name}[done]= 1'd1;"]
        groups[true_branch_name] = true_branch_value.wires + [f'{result_name}.in = {true_branch_value.value};',
                                                              f'{result_name}.write_en = 1\'d1;',
                                                              f'{true_branch_name}[done] = {result_name}.done;']
        groups[false_branch_name] = false_branch_value.wires + [f'{result_name}.in = {false_branch_value.value};',
                                                                f'{result_name}.write_en = 1\'d1;',
                                                                f'{false_branch_name}[done] = {result_name}.done;']

        structures = cond_value.cells + true_branch_value.cells + false_branch_value.cells
        structures.append(result_cell)

        true_branch_name
        return EmitResult(
            f'{result_name}.out',
            None,
            structures,
            [],
            groups,
            [mk_block(f'if {cond_value.value} with {cond_name}',
                      '\n'.join(true_branch_value.controls +
                                [f'{true_branch_name};'])) +
             mk_block('else', '\n'.join(false_branch_value.controls +
                                        [f'{false_branch_name};']))
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
            param_type = param.type_annotation
            dimension, mem_size, mem_index, bitwidth = ExtractTensorTypes(param_type)
            if dimension == 0:
                func_cells.append(f'{name} = prim std_reg({bitwidth});')
            else:
                func_cells.append(f'{name} = prim std_mem_d{dimension}({bitwidth}, {mem_size}, {mem_index});')

        # Make a register for the return value.
        dimension, mem_size, mem_index, bitwidth = ExtractTensorTypes(func.ret_type)
        if dimension == 0:
            func_cells.append(f'ret = prim std_reg({bitwidth});')
        else:
            func_cells.append(f'constant0 = prim std_const({bitwidth}, 0);')
            func_cells.append(f'constant1 = prim std_const({bitwidth}, 1);')
            func_cells.append(f'ret = prim std_mem_d{dimension}({bitwidth}, {mem_size}, {mem_index});')

        # Create a group for the wires that run this expression.
        group_name = 'group{}'.format(id("group"))
        write_enable = body.done if body.wires else f'{group_name}[go]'
        if dimension == 0:
            group_wires = body.wires + [
                f'ret.in = {body.value};',  # FIXME: This works for a single value, but doesn't translate well for
                f'ret.write_en = 1\'d1;',  # a while loop or similar where values are updated on the go.
                f'{group_name}[done] = ret.done;',
            ]
            groups = mk_block(f'group {group_name}', '\n'.join(group_wires))
        else:
            groups = '';

        for group in body.groups.keys():
            groups += '\n' + mk_block(f'group {group}', '\n'.join(body.groups[group]))
        # Construct a FuTIL component. For now, the component is
        # *always* called `main`. Someday, we should actually support
        # multiple functions as multiple components.
        cells = mk_block('cells', '\n'.join(func_cells + body.cells))
        if dimension == 0:
            wires = mk_block('wires', groups)
            control = mk_block('control',
                               mk_block('seq', '\n'.join(body.controls + [f'{group_name};'])))  # Invoke the group.
        else:
            wires = mk_block('wires', groups)
            control = mk_block('control', mk_block('seq', '\n'.join(body.controls)))
        component = mk_block(
            'component main() -> ()',
            '\n'.join([cells, wires, control])
        )

        return component


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
