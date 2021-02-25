import tvm
from tvm import relay
from futil.ast import *
from futil.utils import bits_needed
from typing import List
from dataclasses import dataclass

# Mapping from the tensor dimensions to the
# corresponding FuTIL primitive.
NumDimsToCell = {
    0: Stdlib().register,
    1: Stdlib().mem_d1,
    2: Stdlib().mem_d2,
    3: Stdlib().mem_d3,
    4: Stdlib().mem_d4
}

# Suffix appended to memories by Dahlia when lowering.
DahliaSuffix = {
    'std_const': '',
    'std_reg': '',
    'std_mem_d1': '0',
    'std_mem_d2': '0_0',
    'std_mem_d3': '0_0_0',
    'std_mem_d4': '0_0_0_0'
}


@dataclass
class DahliaFuncDef:
    """Necessary information to compute a Dahlia
    function definition."""
    function_id: str
    dest: CompVar
    args: List[CompVar]
    attributes: tvm.ir.Attrs
    data_type: str


def get_dims(c: CompInst):
    """Mapping from memory to number of dimensions."""
    id = c.id
    id2dimensions = {
        'std_reg': 0,
        'std_mem_d1': 1,
        'std_mem_d2': 2,
        'std_mem_d3': 3,
        'std_mem_d4': 4
    }
    assert id in id2dimensions, f'{id} not supported.'
    return id2dimensions[id]


def get_addr_ports(c: CompInst):
    """Returns a list of ('address, index size)
     for each address port in the component
     instance."""
    args = c.args
    dims = get_dims(c)
    addresses = range(0, dims)
    indices = range(dims + 1, dims << 1 + 1)
    return [
        (f'addr{i}', args[n])
        for (i, n) in zip(addresses, indices)
    ]


def emit_invoke_control(decl: CompVar, dest: Cell, args: List[Cell]) -> Invoke:
    """Returns the Invoke control."""
    in_connects = []
    out_connects = []

    def get_connects(c: Cell, is_destination: bool):
        # Hooks up correct ports for invocation, depending on whether
        # `c` is an argument or a destination memory.
        comp = c.comp
        assert comp.id in DahliaSuffix, f'{comp.id} supported yet.'
        param = f'{c.id.name}{DahliaSuffix[comp.id]}'
        arg = CompVar(c.id.name)

        if any(p in comp.id for p in ['reg', 'const']):
            # If this is a constant or a register.
            return [(f'{param}', CompPort(arg, 'out'))], []

        # Otherwise, its an N-dimensional memory.
        in_, out = [], []
        if is_destination:
            # If the memory is being written to, hook up write ports.
            in_.append(
                (f'{param}_done', CompPort(arg, 'done'))
            )
            out.extend([
                (f'{param}_write_data', CompPort(arg, 'write_data')),
                (f'{param}_write_en', CompPort(arg, 'write_en'))
            ])

        # Reads allowed in either case.
        in_.append(
            (f'{param}_read_data', CompPort(arg, 'read_data'))
        )

        # Hook up address ports.
        addr_ports = [port for port, _ in get_addr_ports(comp)]
        out.extend([
            (f'{param}_{port}', CompPort(arg, f'{port}')) for port in addr_ports
        ])

        return in_, out

    for cell in args:
        # Don't connect write ports for arguments.
        in_, out = get_connects(cell, is_destination=False)
        in_connects.extend(in_)
        out_connects.extend(out)

    dest_in, dest_out = get_connects(dest, is_destination=True)

    return Invoke(
        decl,
        in_connects + dest_in,
        out_connects + dest_out
    )


def get_dahlia_data_type(relay_type) -> str:
    """Gets the Dahlia data type from the given Relay type.
    It maps the types in the following manner:

    Relay   |        Dahlia
    --------|-------------------------------
     int    |   (`bit`, width)
     float  |   (`fix`, (width, width // 2))
    """
    width = get_bitwidth(relay_type)

    if 'int' in relay_type.dtype:
        return f'bit<{width}>'
    if 'float' in relay_type.dtype:
        return f'fix<{width}, {width // 2}>'
    assert 0, f'{relay_type} is not supported.'


def get_bitwidth(relay_type) -> int:
    """Gets the bitwidth from a Relay type.
    """
    dtype = relay_type.dtype
    assert 'int' in dtype or 'float' in dtype, f'{relay_type} not supported.'
    return int(''.join(filter(str.isdigit, dtype)))


def get_memory(name: str, type: tvm.ir.Type) -> Cell:
    """Returns a FuTIL memory for a given TVM type.
    For non-Tensor types, a register is returned.
    Otherwise, a memory with the corresponding dimension size
    is returned, if it exists in FuTIL."""
    dims = type.concrete_shape
    # Bitwidth, along with sizes and index sizes (if it is a Tensor).
    args = [get_bitwidth(type)] + [d for d in dims] + [bits_needed(d) for d in dims]

    num_dims = len(dims)
    assert num_dims in NumDimsToCell, f'Memory of size {num_dims} not supported.'

    return Cell(
        CompVar(name),
        NumDimsToCell[num_dims](*args)
    )


def python2relay(func) -> str:
    """Used to lower Relay IR from the
    TVM Python library."""
    seq = tvm.transform.Sequential([
        relay.transform.SimplifyExpr(),
        relay.transform.SimplifyInference(),
        relay.transform.ToANormalForm(),
    ])
    mod_opt = tvm.IRModule.from_expr(func)
    mod_opt = seq(mod_opt)
    return mod_opt['main']
