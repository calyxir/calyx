# type: ignore
import tvm
from tvm import relay
from calyx.py_ast import CompVar, Stdlib, CompInst, Cell, Invoke, CompPort
from calyx.utils import bits_needed
from typing import List
from dataclasses import dataclass

# Mapping from the tensor dimensions to the
# corresponding Calyx primitive.
NumDimsToCell = {
    0: Stdlib().register,
    1: Stdlib().seq_mem_d1,
    2: Stdlib().seq_mem_d2,
    3: Stdlib().seq_mem_d3,
    4: Stdlib().seq_mem_d4,
}


@dataclass
class DahliaFuncDef:
    """Necessary information to compute a Dahlia
    function definition."""

    function_id: str
    component_name: str
    dest: CompVar
    args: List[CompVar]
    attributes: tvm.ir.Attrs
    data_type: str
    component: CompInst


def get_dims(c: CompInst):
    """Mapping from memory to number of dimensions."""
    id = c.id
    id2dimensions = {
        "std_reg": 0,
        "seq_mem_d1": 1,
        "seq_mem_d2": 2,
        "seq_mem_d3": 3,
        "seq_mem_d4": 4,
    }
    assert id in id2dimensions, f"{id} not supported."
    return id2dimensions[id]


def get_dimension_sizes(c: CompInst) -> List[int]:
    """Given a cell `c`, returns the corresponding
    memory sizes.
    Example:
    comb_mem_d1(32, 8, 3) returns [8]."""
    dims = get_dims(c)
    return [c.args[i] for i in range(1, dims + 1)]


def get_addr_ports(c: CompInst):
    """Returns a list of (address, index size)
    for each address port in the component
    instance."""
    dims = get_dims(c)
    addresses = range(0, dims)
    indices = range(dims + 1, dims << 1 + 1)
    return [(f"addr{i}", c.args[n]) for (i, n) in zip(addresses, indices)]


def emit_invoke_control(
    decl: CompVar, dest: Cell, args: List[Cell], old_args=[], old_dest=None
) -> Invoke:
    """Returns the Invoke control."""
    ref_cells = []
    inputs = []

    def add_arg(cell):
        comp = cell.comp
        param = f"{cell.id.name}"
        arg = CompVar(cell.id.name)

        # If this is a constant or a register, connect the ports
        if any(p in comp.id for p in ["reg", "const"]):
            inputs.append((f"{param}", CompPort(arg, "out")))
        else:
            ref_cells.append((param, arg))

    # this function is similar to add_arg, but is for the case when we are
    # "reusing" a Dahlia Function (which will later be a Calyx component)
    # and therefore need to use the same parameter names as the previous invoke
    def add_arg2(arg_cell, param_cell):
        assert (
            arg_cell.comp == param_cell.comp
        ), "arg cell and param cell must be same component"
        comp = arg_cell.comp
        param = f"{param_cell.id.name}"
        arg = CompVar(arg_cell.id.name)

        # If this is a constant or a register, connect the ports
        if any(p in comp.id for p in ["reg", "const"]):
            inputs.append((f"{param}", CompPort(arg, "out")))
        else:
            ref_cells.append((param, arg))

    if len(old_args) == 0:
        for cell in args:
            add_arg(cell)
        add_arg(dest)
    else:
        # case for when we are "reusing" a Dahlia Function/Calyx component and
        # therefore need to make sure we're using the previous parameter names
        assert len(old_args) == len(
            args
        ), "we are reusing a dahlia function but the args are different lengths"
        assert old_dest is not None, "if using old_args must provide an old_dest too"
        for cell1, cell2 in zip(args, old_args):
            add_arg2(cell1, cell2)
        add_arg2(dest, old_dest)

    return Invoke(decl, inputs, [], ref_cells)


def get_dahlia_data_type(relay_type) -> str:
    """Gets the Dahlia data type from the given Relay type.
    It maps the types in the following manner:

    Relay   |        Dahlia
    --------|-------------------------------
     int    |   (`bit`, width)
     float  |   (`fix`, (width, width // 2))
    """
    width = get_bitwidth(relay_type)

    if "int" in relay_type.dtype:
        return f"bit<{width}>"
    if "float" in relay_type.dtype:
        return f"fix<{width}, {width // 2}>"
    assert 0, f"{relay_type} is not supported."


def get_bitwidth(relay_type) -> int:
    """Gets the bitwidth from a Relay type."""
    dtype = relay_type.dtype
    assert "int" in dtype or "float" in dtype, f"{relay_type} not supported."
    return int("".join(filter(str.isdigit, dtype)))


def get_memory(name: str, type: tvm.ir.Type) -> Cell:
    """Returns a Calyx memory for a given TVM type.
    For non-Tensor types, a register is returned.
    Otherwise, a memory with the corresponding dimension size
    is returned, if it exists in Calyx."""
    dims = type.concrete_shape
    # Bitwidth, along with sizes and index sizes (if it is a Tensor).
    args = [get_bitwidth(type)] + [d for d in dims] + [bits_needed(d) for d in dims]

    num_dims = len(dims)
    assert num_dims in NumDimsToCell, f"Memory of size {num_dims} not supported."

    return Cell(CompVar(name), NumDimsToCell[num_dims](*args), is_external=True)


def python2relay(func) -> str:
    """Used to lower Relay IR from the
    TVM Python library."""
    seq = tvm.transform.Sequential(
        [
            relay.transform.SimplifyExpr(),
            relay.transform.SimplifyInference(),
            relay.transform.ToANormalForm(),
        ]
    )
    mod_opt = tvm.IRModule.from_expr(func)
    mod_opt = seq(mod_opt)
    return mod_opt["main"]
