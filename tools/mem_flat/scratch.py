def mk_naive_addrgen(parent_comp, dim_sizes, idx_sizes, addr_width):
    """
    given a set of dimension sizes and a parent component,
    generate a set of multipliers plus a tree of adders to fill an address.

    mutates parent_comp, returns a handle to the cell containing the address.
    its inputs should be connected by default.
    """
    padders = []
    multipliers = []
    # pad all inputs
    for idx in range(len(dim_sizes)):
        pad_c = parent_comp.pad(
            in_width=idx_sizes[idx],
            out_width=addr_width,
            name=f"padd_addr{idx}",
        )
        with parent_comp.continuous:
            pad_c.in_ = parent_comp.this()[f"addr{idx}"]
        padders.append(pad_c)

    # generate multipliers
    for idx in range(1, len(dim_sizes)):
        addr_mul_c = parent_comp.const_mult(
            size=addr_width, const=dim_sizes[idx], name=f"mul_addr{idx}"
        )
        with parent_comp.continuous:
            addr_mul_c.in_ = padders[idx].out_
        multipliers.append(addr_mul_c)

    # 4 is small enough that the adder tree is done by hand
    if len(dim_sizes) == 2:
        addr_tot = parent_comp.add_use(
            padders[0].out_,
            multipliers[0].out_,
            cellname="int_addr",
            width=addr_width,
        )
        return addr_tot
    elif len(dim_sizes) == 3:
        i1 = parent_comp.add_use(
            padders[0].out_,
            multipliers[0].out_,
            cellname="int_addr_i1",
            width=addr_width,
        )
        addr_tot = parent_comp.add_use(
            i1.cell.out_, multipliers[1].out_, cellname="int_addr", width=addr_width
        )
        return addr_tot

    elif len(dim_sizes) == 4:
        i1 = parent_comp.add_use(
            padders[0].out_,
            multipliers[0].out_,
            cellname="int_addr_i1",
            width=addr_width,
        )
        i2 = parent_comp.add_use(
            multipliers[1].out_,
            multipliers[2].out_,
            cellname="int_addr_i2",
            width=addr_width,
        )
        addr_tot = parent_comp.add_use(
            i1.cell.out_, i2.cell.out_, cellname="int_addr", width=addr_width
        )
        return addr_tot


def add_flatten_mem(prog, data_width, dim_sizes, idx_sizes):
    assert len(dim_sizes) == len(idx_sizes), "dimensions don't match"
    assert 2 <= len(idx_sizes) and len(idx_sizes) <= 4, "dimension count not supported"

    spec = "x".join(str(n) for n in dim_sizes)

    flat_comp = prog.component(f"d{len(idx_sizes)}_flat_{spec}")

    mem_len = 1
    for i in dim_sizes:
        mem_len *= i

    addr_width = clog2_or_1(mem_len)
    # I/O
    inputs = gen_in_lines(idx_sizes)

    inputs.extend(
        [
            ("write_data", data_width),
            ("content_en", 1),
            ("write_en", 1),
            ("reset", 1),
        ]
    )

    outputs = [("read_data", data_width), ("done", 1)]

    add_comp_ports(flat_comp, inputs, outputs)

    # internal address generation
    # addr_mul = flat_comp.const_mult(
    #     size=addr_width, const=dim_sizes[1], name="mul_addr1"
    # )
    # addr_tot = flat_comp.add_use(
    #     flat_comp.this()["addr0"], addr_mul.out_, cellname="int_addr", width=addr_width
    # )

    addr_tot = mk_naive_addrgen(flat_comp, dim_sizes, idx_sizes, addr_width)

    with flat_comp.continuous:
        backing_mem.write_en = flat_comp.this()["write_en"]
        backing_mem.addr0 = addr_tot.cell.out
        backing_mem.write_data = flat_comp.this()["write_data"]
        backing_mem.content_en = flat_comp.this()["content_en"]
        backing_mem.reset = flat_comp.this()["reset"]

        # addr_mul.in_ = flat_comp.this()["addr1"]

        flat_comp.this()["read_data"] = backing_mem.read_data
        flat_comp.this()["done"] = backing_mem.done


# Since yxi is still young, keys and formatting change often.
width_key = "data_width"
size_key = "total_size"
name_key = "name"
