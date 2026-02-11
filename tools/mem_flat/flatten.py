# component to flatten a memory from multi-dimensional to one-dimensional
# these only work on the addresses

from calyx.builder import (
    Builder,
    add_comp_ports,
    invoke,
    while_with,
    par,
    while_,
)

from typing import Literal
from math import log2, ceil
import json
import sys


# generates a 'flattened' memory that works similarly to a higher-dimensioned memory
# the underlying 1d memory can then be written into directly, this is really just a bunch of addressing logic

# TODO address-mapped version provided which may generate more optimal code, but rounds dimensions / indexes to power of 2
# TODO: optimise out multiplies by 1?


def clog2(x):
    """Ceiling log2"""
    if x <= 0:
        raise ValueError("x must be positive")
    return (x - 1).bit_length()


def clog2_or_1(x):
    """Ceiling log2 or 1 if clog2(x) == 0"""
    return max(1, clog2(x))


def gen_in_lines(idx_sizes):
    # generate the input array of tuples
    res = []
    ano = 0

    for sz in idx_sizes:
        res.append((f"addr{ano}", sz, [("write_together(1)")]))
        ano += 1

    return res


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
            name=f"pad{idx}",
        )
        with parent_comp.continuous:
            pad_c.in_ = parent_comp.this()[f"addr{idx}"]
        padders.append(pad_c)

    # generate multipliers
    accum = 1
    for idx in reversed(range(1, len(dim_sizes))):
        accum *= dim_sizes[idx]
        addr_idx = idx - 1
        addr_mul_c = parent_comp.const_mult(
            size=addr_width, const=accum, name=f"mul{addr_idx}"
        )
        with parent_comp.continuous:
            addr_mul_c.in_ = padders[addr_idx].out_
        multipliers.insert(0, addr_mul_c)

    # 4 is small enough that the adder tree is done by hand
    if len(dim_sizes) == 2:
        addr_tot = parent_comp.add(size=addr_width, name="int_addr")
        with parent_comp.continuous:
            addr_tot.left = padders[1].out_
            addr_tot.right = multipliers[0].out_

        return addr_tot
    elif len(dim_sizes) == 3:
        i1 = parent_comp.add(
            name="int_addr_i1",
            size=addr_width,
        )
        addr_tot = parent_comp.add(
            name="addr_tot",
            size=addr_width,
        )
        with parent_comp.continuous:
            i1.left = padders[2].out_
            i1.right = multipliers[0].out_
            addr_tot.left = i1.out_
            addr_tot.right = multipliers[1].out_
        return addr_tot

    elif len(dim_sizes) == 4:
        i1 = parent_comp.add(size=addr_width, name="int_addr_i1")
        i2 = parent_comp.add(size=addr_width, name="int_addr_i2")
        addr_tot = parent_comp.add(size=addr_width, name="int_addr")
        with parent_comp.continuous:
            i1.left = padders[3].out_
            i1.right = multipliers[0].out_
            i2.left = multipliers[1].out_
            i2.right = multipliers[2].out_
            addr_tot.left = i1.out_
            addr_tot.right = i2.out_

        return addr_tot


def add_flatten_mem(prog, width, dim_sizes, idx_sizes):
    assert len(dim_sizes) == len(idx_sizes), "dimensions don't match"
    assert 2 <= len(idx_sizes) and len(idx_sizes) <= 4, "dimension count not supported"

    spec = "x".join(str(n) for n in dim_sizes)

    flat_comp = prog.comb_component(f"d{len(idx_sizes)}_{width}_flat_{spec}")

    mem_len = 1
    for i in dim_sizes:
        mem_len *= i

    addr_width = clog2_or_1(mem_len)
    # I/O
    inputs = gen_in_lines(idx_sizes)
    outputs = [("addr_o", addr_width)]

    add_comp_ports(flat_comp, inputs, outputs)

    addr_tot = mk_naive_addrgen(flat_comp, dim_sizes, idx_sizes, addr_width)
    with flat_comp.continuous:
        flat_comp.this()["addr_o"] = addr_tot.out_


# Since yxi is still young, keys and formatting change often.
width_key = "data_width"
size_key = "total_size"
name_key = "name"


def build():
    prog = Builder(emit_sourceloc=False)
    memsizes = []
    for mem in mems:
        nwidth = mem[width_key]
        nlen = mem[size_key]
        dims = mem["dimension_sizes"]
        if len(dims) == 1:
            continue
        if (nwidth, nlen) not in memsizes:
            add_flatten_mem(prog, nwidth, dims, mem["idx_sizes"])
            memsizes.append((nwidth, nlen))
    # add_main_comp(prog, mems)
    return prog.program


if __name__ == "__main__":
    yxifilename = "input.yxi"  # default
    if len(sys.argv) > 2:
        raise Exception("flattened memory generator takes 1 yxi file name as argument")
    else:
        try:
            yxifilename = sys.argv[1]
            if not yxifilename.endswith(".yxi"):
                raise Exception("flattened memory generator requires an yxi file")
        except Exception:
            pass  # no arg passed
    with open(yxifilename, "r", encoding="utf-8") as yxifile:
        yxifile = open(yxifilename)
        yxi = json.load(yxifile)
        mems = yxi["memories"]
        build().emit()
