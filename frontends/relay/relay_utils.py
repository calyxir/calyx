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
    1: Stdlib().mem_d1,
    2: Stdlib().mem_d2,
    3: Stdlib().mem_d3,
    4: Stdlib().mem_d4,
}

# Suffix appended to memories by Dahlia when lowering.
DahliaSuffix = {
    "std_const": "",
    "std_reg": "",
    "std_mem_d1": "0",
    "std_mem_d2": "0_0",
    "std_mem_d3": "0_0_0",
    "std_mem_d4": "0_0_0_0",
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


def get_dims(c: CompInst):
    """Mapping from memory to number of dimensions."""
    id = c.id
    id2dimensions = {
        "std_reg": 0,
        "std_mem_d1": 1,
        "std_mem_d2": 2,
        "std_mem_d3": 3,
        "std_mem_d4": 4,
    }
    assert id in id2dimensions, f"{id} not supported."
    return id2dimensions[id]


def get_dimension_sizes(c: CompInst) -> List[int]:
    """Given a cell `c`, returns the corresponding
    memory sizes.
    Example:
    std_mem_d1(32, 8, 3) returns [8]."""
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


def emit_invoke_control(decl: CompVar, dest: Cell, args: List[Cell]) -> Invoke:
    """Returns the Invoke control."""
    ref_cells = []

    def get_arg(cell):
        comp = cell.comp
        assert comp.id in DahliaSuffix, f"{comp.id} supported yet."
        param = f"{cell.id.name}{DahliaSuffix[comp.id]}"
        arg = CompVar(cell.id.name)

        return (param, arg)

    for cell in args:
        ref_cells.append(get_arg(cell))

    ref_cells.append(get_arg(dest))

    return Invoke(decl, [], [], ref_cells)


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
