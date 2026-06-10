# components to flatten a memory from multi-dimensional to one-dimensional
# these only work on the addresses

# could be improved by mapping addresses rather than copying memory.

from calyx.builder import (
    Builder,
    add_comp_ports,
    invoke,
    while_with,
    par,
    while_,
)


# handles transfer of data from backing -> working
# TODO: no support for 3 or 4 dimensional sequential memories in calyx python
def add_flatten_in(prog, data_width, mem_size, dim_sizes, idx_sizes):
    assert len(dim_sizes) == len(idx_sizes), "dimensions don't match"

    flat_comp = prog.component(f"m_d{len(dim_sizes)}_flatten_{''.join(dim_sizes)}")

    backing_mem = flat_comp.seq_mem_d1(
        name="backing_mem",
        bitwidth=data_width,
        len=mem_size,
        idx_size=clog2_or_1(mem_size),
        is_external=False,
        is_ref=True,
    )

    if len(dim_sizes) == 2:
        working_mem = flat_comp.seq_mem_d1(
            name="working_mem",
            bitwidth=data_width,
            len=mem_size,
            idx_size=clog2_or_1(mem_size),
            is_external=False,
            is_ref=True,
        )
