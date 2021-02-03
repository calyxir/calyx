from __future__ import annotations  # Used for type annotations.
from futil.ast import *
from futil.utils import bits_needed
from dataclasses import dataclass

# Mapping from the tensor dimensions to the corresponding FuTIL memory.
NumDimsToCell = {
    0: Stdlib().register,
    1: Stdlib().mem_d1,
    2: Stdlib().mem_d2,
    3: Stdlib().mem_d3,
    4: Stdlib().mem_d4
}

# Suffix appended to memories by Dahlia when lowering.
DahliaSuffix = {
    'std_mem_d1': '0',
    'std_mem_d2': '0_0',
    'std_mem_d3': '0_0_0',
    'std_mem_d4': '0_0_0_0'
}


# TODO(cgyurgyik): Necessary with `Invoke` controls?
def dahlia_name(name, component_id):
    """Appends the appropriate suffix for Dahlia codegen.
    Dahlia uses the following naming schema
    for an arbitrary variable `X`:
      Memory1D: `X0`
      Memory2D: `X0_0`
      Memory3D: `X0_0_0`
      Memory4D: `X0_0_0_0`
    """
    assert type in DahliaSuffix, f'{name} with {type} is not supported.'
    return f'{name}{DahliaSuffix}'


@dataclass
class DahliaFuncDef:
    """Necessary information to compute a Dahlia function definition."""
    component_id: CompVar
    function_id: str
    dest: CompVar
    invoke_ctrl: Invoke
    attributes: tvm.ir.Attrs


def get_addr_ports(c: CompInst):
    id = c.id
    args = c.args
    id2dims = {
        'std_mem_d1': 1,
        'std_mem_d2': 2,
        'std_mem_d3': 3,
        'std_mem_d4': 4
    }
    assert id in id2dims.keys(), f'{id} not supported.'
    dims = id2dims[id]
    addresses = range(0, dims)
    indices = range(dims + 1, dims << 1 + 1)
    return [(f'addr{i}', args[n]) for (i, n) in zip(addresses, indices)]


def emit_invoke_control(decl: CompVar, dest: Cell, args: List[Cell]) -> Invoke:
    """Returns the input and output connections for Invoke."""
    in_connects = []
    out_connects = []

    def get_connects(c: Cell):
        comp = c.comp
        assert comp.id in DahliaSuffix, f'{comp.id} supported yet.'
        in_, out = [], []
        param = f'{c.id.name}{DahliaSuffix[comp.id]}'
        arg = CompVar(param)

        # By default, always hook up both read and write ports.
        in_.extend([
            (f'{param}_read_data', CompPort(arg, 'read_data')),
            (f'{param}_done', CompPort(arg, 'done'))
        ])
        out.extend([
            (f'{param}_write_data', CompPort(arg, 'write_data')),
            (f'{param}_write_en', CompPort(arg, 'write_en'))
        ])

        # Hook up address ports.
        addr_ports = [port for port, _ in get_addr_ports(comp)]
        out.extend([
            (f'{param}_{port}', CompPort(arg, f'{port}')) for port in addr_ports
        ])
        return in_, out

    for cell in args + [dest]:
        # We treat the connections of both the destination
        # and argument memories in the same manner for now.
        in_, out = get_connects(cell)
        in_connects.extend(in_)
        out_connects.extend(out)

    return Invoke(
        decl,
        in_connects,
        out_connects
    )


def get_dahlia_data_type(relay_type) -> str:
    """Gets the Dahlia data type from the given Relay type.
    It maps the types in the following manner:

    Relay  | Dahlia
    -------|--------
     int   | (`bit`, width)
     float | (`fix`, (width, width // 2))
    """
    width = get_bitwidth(relay_type)

    if 'int' in relay_type.dtype: return ('bit', width)
    if 'float' in relay_type.dtype: return ('fix', (width, width // 2))
    assert False, f'{relay_type} is not supported.'


def get_bitwidth(relay_type) -> int:
    """Gets the bitwidth from a Relay type.
    """
    dtype = relay_type.dtype
    assert 'int' in dtype or 'float' in dtype, f'{relay_type} not supported.'
    return int(''.join(filter(str.isdigit, dtype)))


def get_memory(name, type) -> Cell:
    """TODO: Document."""
    dims = type.concrete_shape
    # Bitwidth, along with sizes and index sizes (if it is a Tensor).
    args = [get_bitwidth(type)] + [d for d in dims] + [bits_needed(d) for d in dims]

    num_dims = len(dims)
    return Cell(
        CompVar(name),
        NumDimsToCell[num_dims](*args)
    )
