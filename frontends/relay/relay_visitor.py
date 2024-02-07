#!/usr/bin/env python3
# type: ignore
from typing import Tuple

import numpy as np
import tvm
from tvm import relay
from tvm.relay.expr_functor import ExprFunctor
from tvm.relay.function import Function


from collections import defaultdict
from typing import List, Dict

import relay_utils as ru
from calyx.py_ast import (
    Cell,
    CompVar,
    CompInst,
    Import,
    Program,
    SeqComp,
    Stdlib,
    Component,
)
from calyx.utils import float_to_fixed_point
from calyx import numeric_types
from dahlia_impl import emit_components

calyx_keywords_list = ["input"]


def rename_relay_var(name: str) -> str:
    """
    Function to rename relay variable names (that are illegal in Calyx) into legal
    ones. This function is to ensure a consistent standard for renaming, since
    we want to make sure that the cell names in the external memory json match the
    name they are instantiated as in the Calyx file
    """
    new_name = name.replace(".", "_")
    new_name = new_name.replace("/", "_")

    if new_name.isdigit():
        new_name = "var_" + new_name
    if new_name in calyx_keywords_list:
        new_name = "_" + new_name

    return new_name


class Relay2Calyx(ExprFunctor):
    """The main compilation visitor."""

    def __init__(self):
        super(Relay2Calyx, self).__init__()
        self.id_dictionary = defaultdict(int)
        self.function_id_dictionary = defaultdict(int)

        # A dictionary of currently visited variable nodes,
        # since some nodes may be visited more than once.
        self.id_to_cell: Dict[str, Cell] = {}

        # A dictionary of variable names to dimensionality.
        # This used for the data in Calyx simulation.
        self.id_to_shape: Dict[str, Tuple] = {}

        # maps the operator name/ destination memory width to the
        # dahlia function. Used to detect when two dahlia functions are
        # the same so we don't declare it twice
        self.func_def_map = {}

        # For each Relay CallNode, there is an associated
        # Dahlia FuncDef so that it can be lowered from Dahlia
        # to Calyx as a stand-alone component.
        self.func_defs: List[ru.DahliaFuncDef] = []

        # Controls, wires of the main component.
        self.controls = []
        self.wires = []

        self.pos_count = 0

        self.source_map: Dict[str, str] = {}

        # for let stmts such as `let %x13: (_,_) = (%x9, %x12)
        # if %x9 is equal to some memory mem9, and %x12 is equal to some memory mem12
        # this maps the var %x13 -> [mem9, mem12]
        self.tuple_dic = {}

        # for let stmts such as `let %x_10 =  meta[relay.Constant][0]`,
        # which is a multidimensional value, we need a dic to remember such
        # statements so that we can put it in the data json file
        self.mem_data = {}

    def id(self, name):
        """
        Provides a unique identification for a given name.
        If 'a' is seen twice, it will produce: 'a', 'a1'.
        No `_` is used, in accordance with Relay variable
        names.
        """
        id_number = self.id_dictionary[name]
        self.id_dictionary[name] += 1
        return f"{name}{'' if id_number == 0 else id_number}"

    def func_id(self, function_name):
        """Used to uniquely identify functions with the
        same name and arity. Eventually, we'll want to
        instantiante two instances of the same Calyx
        component. For example, if `foo_3x3` is seen twice,
        it will produce: `foo_3x3`, `foo_3x3_1`"""
        id_number = self.id_dictionary[function_name]
        self.id_dictionary[function_name] += 1
        return function_name if id_number == 0 else f"{function_name}_{id_number}"

    def visit_var(self, var) -> list:
        """
        Visits a Relay variable and returns the
        corresponding Calyx memory/memories.
        """
        if var in self.tuple_dic.keys():
            return self.tuple_dic[var]
        if isinstance(var.type_annotation, tvm.ir.type.TupleType):
            # returns a list of names instead
            assert 0, "should have been added to tuple_dic when defined in a let stmt"

        var_id = self.id(rename_relay_var(var.name_hint))
        cell = ru.get_memory(var_id, var.type_annotation)
        if var.type_annotation.concrete_shape:
            # Only add the given variable if it is a tensor.
            self.id_to_shape[var_id] = var.type_annotation.concrete_shape
        self.id_to_cell[var_id] = cell
        return [cell]

    def equivalent_func(self, args1, args2, atts1, atts2):
        """
        Assuming functions 1 and 2 have equivalent destination widths (ex: 1x64x55x55) and
        operator name (ex: "Conv2d"), this function checks if the functions have
        equivalent args and attributes. This is mainly making sure the attributes (so for
        conv2d, things like `padding` or `kernel_size`) and memory sizes of the args
        are the same.
        """
        atts_are_same = True
        if (atts1 is None) != (atts2 is None):
            atts_are_same = False
        if (atts1 is not None) and (atts2 is not None):
            for key in atts1.keys():
                attr1 = atts1.get_str(key)
                attr2 = atts2.get_str(key)
                # even if the contents of tvm.ir.container.Array are the same it
                #  still doesn't return true on '=='
                if isinstance(attr1, tvm.ir.container.Array) and isinstance(
                    attr2, tvm.ir.container.Array
                ):
                    attr1 = list(attr1)
                    attr2 = list(attr2)
                if not attr1 == attr2:
                    atts_are_same = False
        args_are_same = True
        for arg1, arg2 in zip(args1, args2):
            if arg1.comp != arg2.comp:
                args_are_same = False
        return atts_are_same and args_are_same

    def analyze_val_dest(self, let, value, dest, type_annotation):
        """
        Helper method that is ussed to handle certain cases for visiting
        let statements. Should only call when value is a Constant or a Call
        """
        if isinstance(value, tvm.relay.Constant):
            # In the updated version of TVM, sometimes there are assignments
            # in the form of `let %x_10 = meta[relay.Constant][0]`
            # We need to handle remember this data in a dictionary since Calyx
            # will get these values externally in a json file
            for dim_val in value.data.shape:
                if dim_val != 1:
                    np_data = value.data.numpy()
                    self.mem_data[dest.id.name] = np_data
                    return

            # Generates a constant primitive.
            # This is done here since we need
            # both the variable id and the value.
            width = ru.get_bitwidth(value.data)

            if "float" in value.data.dtype:
                # Convert to fixed point.
                constant = float_to_fixed_point(value.data.numpy(), width // 2)
                val = numeric_types.FixedPoint(
                    f"{constant}", width, width // 2, True
                ).unsigned_integer()
            else:
                val = value.data
            cell = Cell(CompVar(dest.id.name), Stdlib.constant(width, val))
            self.id_to_cell[dest.id.name] = cell
        elif isinstance(value, tvm.relay.Call):
            # Generates cells and control for a Relay Call:
            # 1. `Invoke` control
            # 2. Component declaration for the invoked component.
            # 3. `DahliaFuncDef` to generate the Relay call component.

            func_name = value.op.name
            # Function names may have a Relay
            # namespace prepended, e.g. `nn.bias_add`.
            # We want to remove these.
            prefix = func_name.find(".")
            if prefix is not None:
                func_name = func_name[prefix + 1 :]

            # Append arity to Calyx component name.
            dims = "x".join([str(i) for i in ru.get_dimension_sizes(dest.comp)])

            unnested_args = []
            for arg in value.args:
                new_arg = arg
                if isinstance(arg, list):
                    assert (
                        len(arg) == 1
                    ), "only time arg can be a list is when it returns a list of length 1 from visit_var()"
                    new_arg = arg[0]
                unnested_args.append(new_arg)
            value.args = unnested_args

            root_name = f"{func_name}_{dims}"

            is_repeat_func = False

            # If we want to "reuse" a Dahlia function so that we're only generating
            # one Calyx component, when we create the invoke we have
            # to make sure that we use the old names for the parameters
            # (by old names, I mean the names that the previous component used
            # for its ref cell parameters)
            # all of this old_func_args and old_dest stuff will be useful when
            # considering which arguments should be passed into the DahliaFunc/Calyx Invoke
            # statement
            old_func_args = []
            old_dest = None
            if root_name in self.func_def_map:
                for dahlia_func in self.func_def_map[root_name]:
                    if self.equivalent_func(
                        dahlia_func.args,
                        value.args,
                        dahlia_func.attributes,
                        value.attrs,
                    ):
                        # this means we can "reuse" the Dahlia Function which
                        # will later be turned into a Calyx component, since
                        # we have already created a Dahlia function idential
                        # to the one we were about to create
                        comp_name = f"{dahlia_func.component_name}"
                        comp_inst = dahlia_func.component
                        old_func_args = dahlia_func.args
                        old_dest = dahlia_func.dest
                        is_repeat_func = True
                        break

            # Given functions with the same operator and arity,
            # append a unique identifier to the preceding. However, we only want
            # to create a new Dahlia Function (which will be lowered
            # to a Calyx component) if we havnen't encountered the same function
            # before
            if not is_repeat_func:
                comp_name = self.func_id(f"{func_name}_{dims}")
                comp_inst = CompInst(comp_name, [])

            # call self.id on comp_name because now we might produce two instances
            # of the same component
            var_name = self.id(f"{comp_name}_")
            comp_decl = CompVar(f"{var_name}")

            self.id_to_cell[var_name] = Cell(comp_decl, comp_inst)

            # the parameters old_func_args and old_dest are what determines whether
            # ru.emit_invoke_control emits a "new" invoke or an invoke of an already defined
            # Calyx component/Dahlia function
            invoke = ru.emit_invoke_control(
                comp_decl, dest, value.args, old_args=old_func_args, old_dest=old_dest
            )
            invoke.attributes.append(("pos", self.pos_count))
            self.controls.append(invoke)

            tag = self.pos_count
            self.pos_count += 1

            self.source_map[tag] = [
                x for x in str(let).splitlines() if x.startswith("let")
            ][0]

            # only add to Dahlia Functions list and map if we are actually want to
            # use a new Dahlia Function, i.e., if we are not reusing the function
            if not is_repeat_func:
                dahlia_func_def = ru.DahliaFuncDef(
                    function_id=func_name,
                    component_name=comp_name,
                    dest=dest,
                    args=value.args,
                    attributes=value.attrs,
                    data_type=ru.get_dahlia_data_type(type_annotation),
                    component=comp_inst,
                )
                self.func_defs.append(dahlia_func_def)
                if root_name in self.func_def_map:
                    self.func_def_map[root_name].append(dahlia_func_def)
                else:
                    self.func_def_map[root_name] = [dahlia_func_def]

        else:
            assert 0, f"{value} is not supported yet."

    def visit_let(self, let):
        """Visits a `let` statement in the following manner:
        1. Visit the `value`.
        2. Visit the `var`, or destination.
        3. Return the `body`.
        """
        # Check if the dest is a tuple
        if isinstance(let.var.type_annotation, tvm.ir.type.TupleType):
            value = self.visit(let.value)
            # Handles cases such as: `%x13 = (%x9, %x12)`. where %x9 and %x12 will
            # evaluate to cells
            assert isinstance(value, list) and len(value) == len(
                let.var.type_annotation.fields
            ), "Currently, if let destination is a tuple, can only handle 'tuple forwarding' situations"
            unnested_values = []
            # need to do this bc visit_var now returns a list
            for dest in value:
                assert isinstance(dest, list) and isinstance(
                    dest[0], Cell
                ), "Tuples in let value must evaluate to cells"
                unnested_values.append(dest[0])
            # doesn't do anything just increments id by 1 so that we can
            # compare the names the generated Calyx/Dahlia files with the
            # TVM relay more easily.
            self.id(let.var.name_hint)
            # don't need to create new cells, just map the var to the cells in value
            self.tuple_dic[let.var] = unnested_values
        else:
            value = self.visit(let.value)
            dest = self.visit(let.var)
            # need to pass dest[0] bc visit_var returns a list
            self.analyze_val_dest(let, value, dest[0], let.var.type_annotation)
        return self.visit(let.body)

    def visit_tuple(self, tup) -> list:
        """
        For visiting tuple. Just recursively visits each element in the tuple.
        """
        return [self.visit(x) for x in tup.fields]

    def visit_constant(self, const) -> tvm.relay.Constant:
        """Simply returns the Relay constant. Since we don't
        have the variable id here, we generate the Calyx
        cell within the `let` visit."""
        return const

    def visit_call(self, call) -> tvm.relay.Call:
        """The Relay call consists of 3 main pieces:
        call.op, call.args, and call.attrs. The
        latter two are used within call.op.

        call.op is mapped to a corresponding Dahlia function,
        and subsequently lowered to Calyx as a component to
        be invoked.
        """
        # Visit the call arguments.
        call.args = [self.visit(a) for a in call.args]
        # dealing w/ the fact that visit_var returns list
        call.args = flatten_lst(call.args)
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
                name="main",
                inputs=[],
                outputs=[],
                structs=self.wires + list(self.id_to_cell.values()),
                controls=SeqComp(self.controls),
            ),
            self.func_defs,
        )


def flatten_lst(lst):
    """
    Because evaluating a variable sometimes returns a tuple, the return type of
    visit_var() is a list. So when we evaluate a list of variables, we get a
    list of lists back. This function will return a flattened version of
    its input list.
    Precondition: the only elements in lst should be cells and/or lists of
    cells
    """
    flat = []
    for elt in lst:
        if isinstance(elt, Cell):
            flat.append(elt)
        elif isinstance(elt, list):
            for sub_elt in elt:
                flat.append(sub_elt)
        else:
            assert 0, "Args must evaluate to a Cell"
    return flat


def relay_transforms(mod) -> Function:
    """https://tvm.apache.org/docs/api/python/relay/transform.html"""
    transforms = tvm.transform.Sequential(
        [
            relay.transform.SimplifyExpr(),
            relay.transform.SimplifyInference(),
        ]
    )
    if isinstance(mod, relay.Function):
        mod = tvm.IRModule.from_expr(mod)
    mod = transforms(mod)

    return mod["main"]


def check_naming_convention(func_defs: List[ru.DahliaFuncDef]):
    """Names that begin with the prefix `__` are reserved for
    the Dahlia programs that are created to implement the
    respective Relay call nodes. For example, `__x` is
    not allowed, but `_x` and `x` are OK.
    """

    def is_reserved(x):
        return x[:2] == "__"

    for f in func_defs:
        variables = [v.id.name for v in f.args + [f.dest]]
        reserved_variables = list(filter(is_reserved, variables))
        if reserved_variables:
            raise Exception(
                f"Relay call node: `{f.function_id}` violates the naming convention. No "
                "variables should be prefixed with `__`. This is reserved for Dahlia "
                "local variables used before lowering to Calyx. Offending variable name(s): "
                f"{', '.join(reserved_variables)}"
            )


def emit_calyx(relay_ir, save_mem=True) -> (str, Program):
    """Lowers a Relay function to a Calyx program."""
    relay_ir = relay_transforms(relay_ir)
    visitor = Relay2Calyx()
    main, func_defs = visitor.visit(relay_ir)
    check_naming_convention(func_defs)

    return (
        emit_components(func_defs, save_mem),
        Program(
            imports=[
                # Manually printed because we need to print the Dahlia
                # function definitions
            ],
            components=[main],
            meta=visitor.source_map,
        ),
    )


def get_program_dat_memories(relay_ir):
    """Returns a mapping (id -> tensor size)
    for each memory in the Relay IR. The format
    explicitly follows the `dat` format; this
    is used for Calyx simulation."""
    visitor = Relay2Calyx()
    relay_ir = relay_transforms(relay_ir)
    _, func_defs = visitor.visit(relay_ir)

    memories = {}
    for id, shape in visitor.id_to_shape.items():
        if id in visitor.mem_data.keys():
            memories[id] = {
                "data": visitor.mem_data[id].tolist(),
                "format": {
                    "numeric_type": "fixed_point",
                    "is_signed": True,
                    "width": 32,
                    "frac_width": 16,
                },
            }
        else:
            memories[id] = {
                "data": np.zeros(shape).tolist(),
                "format": {
                    "numeric_type": "fixed_point",
                    "is_signed": True,
                    "width": 32,
                    "frac_width": 16,
                },
            }

    return memories


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Lower Relay IR to Calyx.")
    parser.add_argument("file", help="Path to the Relay IR.")
    parser.add_argument(
        "-s",
        "--save_mem",
        required=False,
        help="boolean to determine whether you the Calyx design to use less memory",
    )

    args = parser.parse_args()
    if args.file is None:
        raise Exception(
            "The TVM Relay visitor requires a file containing the Relay IR."
        )

    with open(args.file, "r") as file:
        relay_ir = file.read()
    assert (
        '#[version = "0.0.5"]' in relay_ir
    ), 'TVM Requires #[version = "0.0.5"] at the top of the Relay IR file.'

    relay_ir = tvm.parser.fromtext(relay_ir)

    imports = [
        Import("primitives/core.futil"),
        Import("primitives/memories/seq.futil"),
        Import("primitives/binary_operators.futil"),
        Import("primitives/math.futil"),
    ]

    # save_mem is an optional argument. If user doesn't specify, we
    # want default to be save_mem = true
    save_mem = (
        args.save_mem == "true" or args.save_mem == "True" or args.save_mem is None
    )

    (dahlia_defs, prog) = emit_calyx(relay_ir, save_mem)
    for imp in imports:
        print(imp.doc())
    print(dahlia_defs)
    print(prog.doc())
