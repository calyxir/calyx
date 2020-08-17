from tvm import relay, ir
from tvm.relay.expr_functor import ExprFunctor
from tvm.relay.function import Function
import textwrap
from collections import namedtuple
import math

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

def extract_tensor_params(tensor_type):
    dimension = tensor_type.shape
    mem_size_params = ",".join([str(d) for d in dimension])
    mem_index_params = ",".join([str(int(math.log2(d.__int__()))) for d in dimension])
    return len(dimension), mem_size_params, mem_index_params

class Relay2Futil(ExprFunctor):
    """Our main compilation visitor.
    """
    def __init__(self):
        super(Relay2Futil, self).__init__()
        self.next_id = 0
        self.is_ret = True

    def fresh_id(self):
        the_id = self.next_id
        self.next_id += 1
        return the_id

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
        self.is_ret = False
        # construct a cell for the variable
        name_var = let.var.name_hint
        cell = [f'{name_var} = prim std_reg(32);']
        # start a new group
        group_name = f'group{self.fresh_id()}'
        # visit value expr
        expr_value = self.visit(let.value)
        # add a wire from value.out to name_var
        wires = [f'{name_var}.in = {expr_value.value}', f"{name_var}.write_en = 1'd1", f'{group_name}[done] = {name_var}.done'] + expr_value.wires
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
            body_value.controls + [f'{group_name}']
        )

    def visit_call(self, call):
        # Visit the arguments to the call, emitting their control
        # statements.
        arg_stmts = [self.visit(arg) for arg in call.args]
        #currently assume we only have 2 args to add
        structures = [item for arg in arg_stmts for item in arg.cells]
        wires = [item for arg in arg_stmts for item in arg.wires]
        controls = [item for arg in arg_stmts for item in arg.controls]
        groups = {}
        for arg in arg_stmts:
            groups.update(arg.groups)
        #map standard relay call to hw name in futil
        built_in_calls = {'add':'add', 'subtract':'sub', 'equal':'eq'}
        if call.op.name in built_in_calls:
            arg1_type = call.args[0].checked_type
            arg2_type = call.args[1].checked_type
            dimension_arg1, mem_size_arg1, mem_index_arg1 = extract_tensor_params(arg1_type)
            dimension_arg2, mem_size_arg2, mem_index_arg2 = extract_tensor_params(arg2_type)
            #make sure the left and right of add have the same dimensions
            assert dimension_arg1 == dimension_arg2 and mem_size_arg1 == mem_size_arg2
            # handle the scalar case
            if dimension_arg1 == 0:
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
                        groups,
                        []
                    )
            #build a return memory
            mem_cell_name = 'ret' if self.is_ret else  f'mem{self.fresh_id()}'
            mem_cell = f'{mem_cell_name} = prim std_mem_d{dimension_arg1}(32, {mem_size_arg1}, {mem_index_arg1})'
            hw_type = built_in_calls[call.op.name]
            hw_cell_name = f'{hw_type}{self.fresh_id()}'
            hw_cell = '{} = prim std_{}({});'.format(
                hw_cell_name,
                hw_type,
                32,     # Bit width.
            )
            const_mem_size_name = f'const{self.fresh_id()}'
            const_mem_size = f'{const_mem_size_name} = prim std_const(32, {mem_size_arg1})'

            index_reg_name = f'i{self.fresh_id()}'
            index_reg = f'{index_reg_name} = prim std_reg(32)'
            
            update_adder_name = f'add{self.fresh_id()}'
            update_adder = '{} = prim std_add({});'.format(
                hw_cell_name,
                32,     # Bit width.
            )

            less_comparator_name = f'le{self.fresh_id()}'
            less_comparator = f'{less_comparator_name} = prim std_le(32)'
            structures.extend([hw_cell, mem_cell, const_mem_size, index_reg, less_comparator])
            
            condition_name = f'cond{self.fresh_id()}'
            groups[condition_name] = [
                    f"{condition_name}[done] = 1'd1",
                    f"{less_comparator_name}.left = {index_reg_name}.out",
                    f"{less_comparator_name}.left = {const_mem_size_name}.out"
                    ]
            initialization_name = f'initalize{self.fresh_id()}'
            groups[initialization_name] = [
                    f"{index_reg_name}.in = constant0.out",
                    f"{index_reg_name}.write_en = 1'd1",
                    f"{initialization_name}[done] = {index_reg_name}.done"
                    ]
            add_body_name = f'body{self.fresh_id()}'
            groups[add_body_name] = [
                    f"{mem_cell_name}.addr0 = {index_reg_name}.out",
                    f"{mem_cell_name}.write_en = 1'd1",
                    f"{hw_cell_name}.left = {arg_stmts[0].value}.read_data",
                    f"{hw_cell_name}.right = {arg_stmts[1].value}.read_data",
                    f"{arg_stmts[0].value}.addr0 = {index_reg_name}.out",
                    f"{arg_stmts[1].value}.addr0 = {index_reg_name}.out",
                    f"{mem_cell_name}.write_data = 1'd1 ? {hw_cell_name}.out",
                    f"{add_body_name}[done] = {mem_cell_name}.done ? 1'd1"
                    ]

            update_name = f'update{self.fresh_id()}'
            groups[update_name] = [
                    f"{index_reg_name}.write_en = 1'd1",
                    f"{update_adder_name}.left = {index_reg_name}.out",
                    f"{update_adder_name}.right = constant1.out",
                    f"{index_reg_name}.in = 1'd1 ? {update_adder_name}.out",
                    f"{update_name}[done] = {index_reg_name}.done ? 1'd1"
                    ]
            
            seq_block = mk_block("seq", "\n".join([add_body_name, update_name]))
            mem_control =  mk_block(f"while le0.out with cond0", f'{initialization_name}\n{seq_block}')
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
        cond_name = f'cond{self.fresh_id()}'
        # Process true branch
        true_branch_value = self.visit(if_else.true_branch)
        true_branch_name = f'branch{self.fresh_id()}'
        # Process false branch
        false_branch_value = self.visit(if_else.false_branch)
        false_branch_name = f'branch{self.fresh_id()}'
        # Update groups map
        result_name = f'res{self.fresh_id()}'
        result_cell = '{} = prim std_reg({});'.format(
            result_name,
            32,     # Bit width.
        )
    
        groups = {**true_branch_value.groups, **false_branch_value.groups} 
        groups[cond_name] = cond_value.wires + [f"{cond_name}[done]= 1'd1"]
        groups[true_branch_name] = true_branch_value.wires + [f'{result_name}.in = {true_branch_value.value}', f'{true_branch_name}[done] = {result_name}.done']
        groups[false_branch_name] = false_branch_value.wires + [f'{result_name}.in = {false_branch_value.value}', f'{false_branch_name}[done] = {result_name}.done']

        structures  = cond_value.cells + true_branch_value.cells +  false_branch_value.cells
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
            func_cells.append(f'constant0 = prim std_const(32, 0)')
            func_cells.append(f'constant1 = prim std_const(32, 1)')
            func_cells.append(f'ret  = prim std_mem_d{dimension}(32, {mem_size}, {mem_index});')

        
        # Create a group for the wires that run this expression.
        group_name = 'group{}'.format(self.fresh_id())
        write_enable = body.done if body.wires else f'{group_name}[go]'
        group_wires = body.wires + [
            f'ret.in = {body.value};',
            f'ret.write_en = {write_enable};',
            f'{group_name}[done] = ret[done];',
        ]
        
        groups = mk_block(f'group {group_name}', '\n'.join(group_wires))
        for group in body.groups.keys():
            groups += '\n' + mk_block(f'group {group}', '\n'.join(body.groups[group]))
        # Construct a FuTIL component. For now, the component is
        # *always* called `main`. Someday, we should actually support
        # multiple functions as multiple components.
        cells = mk_block('cells', '\n'.join(func_cells + body.cells))
        wires = mk_block('wires', groups)
        control = mk_block('control', mk_block('seq', '\n'.join(body.controls + [f'{group_name}'])))  # Invoke the group.
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
