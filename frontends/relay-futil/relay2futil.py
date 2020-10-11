from tvm import relay, ir
from tvm.relay.expr_functor import ExprFunctor
from tvm.relay.function import Function
import textwrap
from collections import namedtuple
import math

PREAMBLE = """import "primitives/std.lib";"""

# Map standard relay call to respective hardware name in FuTIL.
BuiltInCalls = {'add': 'add', 'subtract': 'sub', 'equal': 'eq'}

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


# Extracts 1-dimensional tensor parameters.
# dimension should contain two parameters R, C where R == 1.
#
# Example:
# x = relay.var("x", relay.TensorType((1, 4), "int32"))
# extract_1D_tensor_params(x) ->
#   len(dimension):  2
#   mem_size:        4
#   mem_index:       2
#
# TODO(cgyurgyik): Currently, bitwidth is defaulted to 32. Add bitwidth in a follow-up CL.
#                  Hit N-dimensional case. I believe we're limited to 3?
#                  Generalize this for 0-dimensional tensors as well (i.e. scalars).
def extract_tensor_params(tensor_type):
    dimension = tensor_type.shape
    type = tensor_type.dtype
    if len(dimension) == 0:  # Scalar.
        return 0, "", ""

    assert (dimension[0] == 1), "This should be tensor of rank 1, i.e. a vector."
    mem_size = dimension[1]  # Number of columns
    mem_index = str(int(math.log2(dimension[1].__int__())))
    assert (int(''.join(filter(str.isdigit, type))) == 32), "Bitwidths are currently hardcoded to 32."
    return dimension[0], mem_size, mem_index  # , bitwidth


class Relay2Futil(ExprFunctor):
    """Our main compilation visitor.
    """

    def __init__(self):
        super(Relay2Futil, self).__init__()
        self.id_dictionary = {'cond': 0, 'const': 0, 'control': 0, 'group': 0, 'let': 0, 'seq': 0, 'std_add': 0,
                              'std_le': 0, 'std_mem_d1': 0, 'std_mem_d2': 0, 'std_mem_d3': 0, 'std_reg': 0, }
        self.is_ret = True

    def fresh_id(self, element):
        assert (element in self.id_dictionary), 'Add this element to the id_dictionary.'
        id = self.id_dictionary[element]
        self.id_dictionary[element] += 1
        return id

    def visit_var(self, var):
        name = var.name_hint
        dimension, mem_size, mem_index = extract_tensor_params(var.type_annotation)
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
        name = 'const{}'.format(self.fresh_id('const'))
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
        group_name = f'group{self.fresh_id("group")}'
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
        # Visit the arguments to the call, emitting their control
        # statements.
        arg_stmts = [self.visit(arg) for arg in call.args]
        # currently assume we only have 2 args to add
        structures = [item for arg in arg_stmts for item in arg.cells]
        wires = [item for arg in arg_stmts for item in arg.wires]
        controls = [item for arg in arg_stmts for item in arg.controls]
        groups = {}
        for arg in arg_stmts:
            groups.update(arg.groups)
        if call.op.name in BuiltInCalls:
            arg1_type = call.args[0].checked_type
            arg2_type = call.args[1].checked_type
            dimension_arg1, mem_size_arg1, mem_index_arg1 = extract_tensor_params(arg1_type)
            dimension_arg2, mem_size_arg2, mem_index_arg2 = extract_tensor_params(arg2_type)
            # Verify left and right of add have the same dimensions.
            assert dimension_arg1 == dimension_arg2 and mem_size_arg1 == mem_size_arg2
            # Scalar case.
            if dimension_arg1 == 0:
                hw_type = BuiltInCalls[call.op.name]
                # Create structure for an adder.
                cell_name = f'{hw_type}{self.fresh_id("std_add")}'
                cell = f'{cell_name} = prim std_{hw_type}({32});'
                structures.append(cell)
                wires.extend([
                    f'{cell_name}.left = {arg_stmts[0].value};',
                    f'{cell_name}.right = {arg_stmts[1].value};'
                ])
                return EmitResult(
                    f'{cell_name}.out',
                    f"1'd1",
                    structures,
                    wires,
                    groups,
                    []
                )
            # 1-D tensor case.
            # Build a return memory
            mem_cell_name = 'ret' if self.is_ret else f'mem{self.fresh_id("std_mem_d1")}'
            if self.is_ret:
                self.is_ret = False
            else:
                mem_cell = f'{mem_cell_name} = prim std_mem_d{dimension_arg1}(32, {mem_size_arg1}, {mem_index_arg1})'
                structures.append(mem_cell)

            hw_op_type = BuiltInCalls[call.op.name]
            hw_op_cell_name = f'{hw_op_type}{self.fresh_id("std_" + hw_op_type)}'
            hw_op_cell = f'{hw_op_cell_name} = prim std_{hw_op_type}({32});'

            const_mem_size_name = f'mem_size_const{self.fresh_id("const")}'
            const_mem_size = f'{const_mem_size_name} = prim std_const({32}, {mem_size_arg1});'

            const_begin_array_address_name = f'begin_array_const'
            const_begin_of_array_addr = f'{const_begin_array_address_name} = prim std_const({mem_index_arg1}, {0});'

            const_end_array_address_name = f'end_array_const'
            const_end_of_array_addr = f'{const_end_array_address_name} = prim std_const({mem_index_arg1}, {mem_size_arg1 - 1});'

            index_reg_name = f'i'
            index_reg = f'{index_reg_name} = prim std_reg({mem_index_arg1});'

            const_increment_name = f'incr'
            const_increment = f'{const_increment_name} = prim std_const({mem_index_arg1}, {1});'

            update_address_name = f'address_add'
            update_address = f'{update_address_name} = prim std_add({mem_index_arg1});'

            less_comparator_name = f'le'
            less_comparator = f'{less_comparator_name} = prim std_le({mem_index_arg1});'
            structures.extend([hw_op_cell, const_mem_size, index_reg, less_comparator, update_address,
                               const_begin_of_array_addr, const_end_of_array_addr, const_increment])

            condition_name = f'cond{self.fresh_id("cond")}'
            groups[condition_name] = [
                f"{less_comparator_name}.left = {index_reg_name}.out;",
                f"{less_comparator_name}.right = {const_end_array_address_name}.out;",
                f"{condition_name}[done] = 1'd1;"
            ]
            initialization_name = f'let{self.fresh_id("let")}'
            groups[initialization_name] = [
                f"{index_reg_name}.in = {const_begin_array_address_name}.out;",
                f"{index_reg_name}.write_en = 1'd1;",
                f"{initialization_name}[done] = {index_reg_name}.done;"
            ]
            add_body_name = f'body{self.fresh_id("group")}'
            groups[add_body_name] = [
                f"{mem_cell_name}.addr0 = {index_reg_name}.out;",
                f"{mem_cell_name}.write_en = 1'd1;",
                f"{arg_stmts[0].value}.addr0 = {index_reg_name}.out;",
                f"{arg_stmts[1].value}.addr0 = {index_reg_name}.out;",
                f"{hw_op_cell_name}.left = 1'd1 ? {arg_stmts[0].value}.read_data;",
                f"{hw_op_cell_name}.right = 1'd1 ? {arg_stmts[1].value}.read_data;",
                f"{mem_cell_name}.write_data = {hw_op_cell_name}.out;",
                f"{add_body_name}[done] = {mem_cell_name}.done ? 1'd1;"
            ]

            update_name = f'update{self.fresh_id("group")}'
            groups[update_name] = [
                f"{index_reg_name}.write_en = 1'd1;",
                f"{update_address_name}.left = {index_reg_name}.out;",
                f"{update_address_name}.right = {const_increment_name}.out;",
                f"{index_reg_name}.in = 1'd1 ? {update_address_name}.out;",
                f"{update_name}[done] = {index_reg_name}.done ? 1'd1;",
            ]

            seq_block = mk_block("seq", "\n".join([f'{add_body_name};', f'{update_name};']))
            mem_control = mk_block(f"while {less_comparator_name}.out with {condition_name}", f'{seq_block}')
            controls.append(f'{initialization_name};')
            controls.append(mem_control)
            return EmitResult(
                f'{mem_cell_name}',
                None,
                structures,
                wires,
                groups,
                controls
            )

    def visit_if(self, if_else):
        # Process conditions
        cond_value = self.visit(if_else.cond)
        cond_name = f'cond{self.fresh_id("cond")}'
        # Process true branch
        true_branch_value = self.visit(if_else.true_branch)
        true_branch_name = f'branch{self.fresh_id("group")}'
        # Process false branch
        false_branch_value = self.visit(if_else.false_branch)
        false_branch_name = f'branch{self.fresh_id("group")}'
        # Update groups map
        result_name = f'res{self.fresh_id("std_reg")}'
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
            dimension, mem_size, mem_index = extract_tensor_params(param_type)
            if dimension == 0:
                func_cells.append(f'{name} = prim std_reg(32);')
            else:
                func_cells.append(f'{name} = prim std_mem_d{dimension}(32, {mem_size}, {mem_index});')

        # Make a register for the return value.
        dimension, mem_size, mem_index = extract_tensor_params(func.ret_type)
        if dimension == 0:
            func_cells.append('ret = prim std_reg(32);')
        else:
            func_cells.append(f'constant0 = prim std_const(32, 0);')
            func_cells.append(f'constant1 = prim std_const(32, 1);')
            func_cells.append(f'ret = prim std_mem_d{dimension}(32, {mem_size}, {mem_index});')

        # Create a group for the wires that run this expression.
        group_name = 'group{}'.format(self.fresh_id("group"))
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
