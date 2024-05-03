import json
import sys

from typing import Dict, List, Tuple
from calyx.py_ast import (
    CompVar,
    Stdlib,
    SeqComp,
    CompPort,
    Enable,
    While,
    ParComp,
    Control,
    Empty,
)
from . import ast
import calyx.builder as cb
from . import map as map_impl


class CompileError(Exception):
    """Compilation failed; recovery was impossible."""


def cond_group(
    comp: cb.ComponentBuilder, idx: cb.CellBuilder, arr_size: int, suffix: str
) -> Tuple[str, str]:
    """
    Creates a group that checks if the index is less than the array size.
    """
    # ANCHOR: cond_group
    group_name = f"cond_{suffix}"
    cell = f"lt_{suffix}"
    less_than = comp.cell(cell, Stdlib.op("lt", 32, signed=False))
    with comp.comb_group(group_name):
        less_than.left = idx.out
        less_than.right = arr_size
    # ANCHOR_END: cond_group

    return cell, group_name


def incr_group(comp: cb.ComponentBuilder, idx: cb.CellBuilder, suffix: str) -> str:
    """
    Creates a group that increments the index.
    """
    # ANCHOR: incr_group
    group_name = f"incr_idx_{suffix}"
    adder = comp.add(32)
    with comp.group(group_name) as incr:
        adder.left = idx.out
        adder.right = 1
        idx.in_ = adder.out
        idx.write_en = 1
        incr.done = idx.done
    # ANCHOR_END: incr_group

    return group_name


def incr_init_group(comp: cb.ComponentBuilder, idx: cb.CellBuilder, suffix: str) -> str:
    """
    Creates a group that increments the index.
    """
    # ANCHOR: incr_init_group
    group_name = f"init_idx_{suffix}"
    with comp.group(group_name) as incr:
        idx.in_ = 0
        idx.write_en = 1
        incr.done = idx.done
    # ANCHOR_END: incr_init_group

    return group_name


def gen_reduce_impl(
    comp: cb.ComponentBuilder, dest: str, stmt: ast.Reduce, arr_size: int, s_idx: int
):
    """
    Implements a `reduce` statement of the form:
        out := reduce 1 (acc, a <- avec) init { acc + a }
    The implementation first initializes the accumulator to `init` and then
    directly accumulates the values of the array into the accumulator.
    """
    idx = comp.reg(32, f"idx_{s_idx}")

    # Initialize the idx register
    incr_init = incr_init_group(comp, idx, f"{s_idx}")
    # Increment the index register
    incr = incr_group(comp, idx, f"{s_idx}")
    # Check if we've reached the end of the loop
    (port, cond) = cond_group(comp, idx, arr_size, f"{s_idx}")

    # Perform the computation
    assert (
        len(stmt.binds) == 1
    ), "Reduce statements with multiple bind clauses are not supported"

    # Split up the accumulator and the array element
    bind = stmt.binds[0]
    [acc, ele] = bind.dst

    # The source of a `reduce` must be a singly-banked array (thus the `b0`)
    # The destination of a `reduce` must be a register
    name2arr = {acc: f"{dest}_reg", ele: f"{bind.src}_b0"}
    name2outwire = {acc: "out", ele: "read_data"}

    def expr_to_port(expr: ast.BaseExpr):
        if isinstance(expr, ast.LitExpr):
            return cb.const(32, expr.value)
        if isinstance(expr, ast.VarExpr):
            return CompPort(CompVar(name2arr[expr.name]), name2outwire[expr.name])
        raise CompileError(f"Unhandled expression: {type(expr)}")

    try:
        out = comp.get_cell(f"{dest}_reg")  # The accumulator is a register
    except Exception as exc:
        raise TypeError(
            "The accumulator of a `reduce` operation is expected to be a "
            "register. Consider checking the declaration of variable "
            f"`{dest}`."
        ) from exc

    # Initialize the accumulator to `init`.
    init = f"init_{s_idx}"
    init_val = stmt.init
    assert isinstance(init_val, ast.LitExpr), "Reduce init must be a literal"
    with comp.group(init) as group:
        out.in_ = init_val.value
        out.write_en = 1
        group.done = out.done

    body = stmt.body

    if not isinstance(body, ast.BinExpr):
        raise NotImplementedError("Reduce body must be a binary expression")

    if body.operation == "mul":
        operation = comp.cell(f"mul_{s_idx}", Stdlib.op("mult_pipe", 32, signed=False))
    else:
        operation = comp.add(32)
    with comp.group(f"reduce{s_idx}") as evl:
        inp = comp.get_cell(f"{bind.src}_b0")
        inp.addr0 = idx.out
        operation.left = expr_to_port(body.lhs)
        operation.right = expr_to_port(body.rhs)
        out.in_ = operation.out
        # Multipliers are sequential so we need to manipulate go/done signals
        if body.operation == "mul":
            operation.go = 1
            out.write_en = operation.done
        else:
            out.write_en = 1
        evl.done = out.done

    control = SeqComp(
        [
            ParComp([Enable(init), Enable(incr_init)]),
            While(
                port=CompPort(CompVar(port), "out"),
                cond=CompVar(cond),
                body=SeqComp([Enable(f"reduce{s_idx}"), Enable(incr)]),
            ),
        ]
    )

    return control


# ANCHOR: my_map_impl
def my_map_impl(
    comp: cb.ComponentBuilder,
    dest: str,
    stmt: ast.Map,
    arr_size: int,
    bank_factor: int,
    s_idx: int,
):
    """
    Returns a dictionary containing Calyx cells, wires and
    control needed to implement a `map` statement.
    See gen_stmt_impl for format of the dictionary.

    Generates these groups:
      - a group that implements the body of the `map` statement
      - a group that increments an index to access the `map` input array
      - a group that implements the loop condition, checking if the index
        has reached the end of the input array
    """
    # TODO: Implement map!
    return Empty()
    # ANCHOR_END: my_map_impl


def gen_stmt_impl(
    comp: cb.ComponentBuilder,
    stmt: ast.Stmt,
    arr_size: int,
    name2par: Dict[str, int],
    statement_idx: int,
    use_my_map_impl: bool,
) -> Control:
    """
    Returns Calyx cells, wires, and control needed to implement
    a MrXL `map` or `reduce` statement. It is a dictionary
    of this form:
    {
        "cells": <list of strings containing cell defs>,
        "wires": <list of strings containing wire defs>,
        "control": <list of strings containing control statements>
    }

    s_idx is the "statement index." The first `map` or `reduce`
    statement has s_idx=0, and this is suffixed at the end of
    each cell used to implement this statement. This number
    is incremented for each subsequent statement.

    name2par maps memory names to banking factors.
    """
    if isinstance(stmt.operation, ast.Map):
        gen_map_fn = my_map_impl if use_my_map_impl else map_impl.gen_map_impl
        return gen_map_fn(
            comp,
            stmt.dst,
            stmt.operation,
            arr_size,
            name2par[stmt.dst],
            statement_idx,
        )
    else:
        return gen_reduce_impl(comp, stmt.dst, stmt.operation, arr_size, statement_idx)


def compute_par_factors(stmts: List[ast.Stmt]) -> Dict[str, int]:
    """Maps the name of memories to their banking factors."""
    out: Dict[str, int] = dict()

    def add_par(mem: str, par: int):
        # If we've already inferred a banking factor for this memory,
        # make sure it's the same as the one we're inferring now.
        if mem in out and par != out[mem]:
            raise CompileError(
                f"Previous uses of `{mem}` have caused it to have "
                f"banking factor {out[mem]} "
                f"but the current use requires banking factor {par}"
            )
        out[mem] = par

    for stmt in stmts:
        par_f = stmt.operation.par
        if isinstance(stmt.operation, ast.Map):
            add_par(stmt.dst, par_f)  # The destination of a `map` is a vector
        elif par_f != 1:
            raise NotImplementedError("Reduction does not support parallelism")
        for bind in stmt.operation.binds:
            add_par(bind.src, par_f)

    return out


def get_output_data(decls: List[ast.Decl]) -> Dict[str, int]:
    """
    Return a dictionary mapping the variable name of each output variable to its size
    """
    output_data: dict[str, int] = {}
    for decl in decls:
        if not decl.input:
            size = decl.type.size
            size = size if size else 1
            # `size = None` is used to signify a register.
            # For the present purpose, it needs to have size 1.
            output_data[decl.name] = size
    return output_data


def emit_data(prog: ast.Prog, data):
    """
    Return a string containing futil input for `prog`, inferred from `data`
    """
    output_vars = get_output_data(prog.decls)
    for var, size in output_vars.items():
        data[var] = [0] * size
    par_factors = compute_par_factors(prog.stmts)
    calyx_data = dict()
    for var, val in data.items():
        banking_factor = par_factors.get(var)
        if banking_factor:
            bank_size = len(val) // banking_factor
            for i in range(banking_factor):
                bank = f"{var}_b{i}"
                calyx_data[bank] = {
                    "data": val[(i * bank_size) : ((i + 1) * bank_size)],
                    "format": {
                        "numeric_type": "bitnum",
                        "is_signed": False,
                        "width": 32,
                    },
                }
        else:
            calyx_data[var] = {
                "data": val,
                "format": {"numeric_type": "bitnum", "is_signed": False, "width": 32},
            }
    json.dump(calyx_data, sys.stdout, indent=4, sort_keys=True)


def reg_to_mem_group(
    comp: cb.ComponentBuilder, var: str, reg: cb.CellBuilder, mem: cb.CellBuilder
) -> str:
    """
    Creates a group that increments the index.
    """
    # ANCHOR: reg2mem_group
    group_name = f"{var}_reg2mem"
    with comp.group(group_name) as reg2mem:
        mem.addr0 = 0
        mem.write_data = reg.out
        mem.write_en = 1
        reg2mem.done = mem.done
    # ANCHOR_END: reg2mem_group

    return group_name


def emit(prog: ast.Prog, use_my_map_impl: bool = False):
    """
    Returns a string containing a Calyx program, compiled from `prog`, a MrXL
    program.
    """

    # Instantiate a Calyx program
    calyx_prog = cb.Builder()
    main = calyx_prog.component("main")

    # All arrays must be the same size. The first array we see determines the
    # size that we'll assume for the rest of the program's arrays.
    arr_size = None

    # ANCHOR: compute_par_factors
    # Collect banking factors.
    par_factor = compute_par_factors(prog.stmts)
    # ANCHOR_END: compute_par_factors

    # Collect memory and register declarations.
    used_names = []
    reg_to_mem = []
    for decl in prog.decls:
        used_names.append(decl.name)
        if decl.type.size:  # A memory
            # Ensure all memories have the same size
            if not arr_size:
                arr_size = decl.type.size
            elif arr_size != decl.type.size:
                raise CompileError(
                    f"Memory `{decl.name}` has size {decl.type.size}"
                    f" but previous memory had size {arr_size}"
                )
            name = decl.name
            par = par_factor[name]
            # ANCHOR: collect-decls
            for i in range(par):
                main.comb_mem_d1(
                    f"{name}_b{i}", 32, arr_size // par, 32, is_external=True
                )
            # ANCHOR_END: collect-decls
        else:  # A register
            name = decl.name
            mem = main.comb_mem_d1(name, 32, 1, 32, is_external=True)
            reg = main.reg(
                32,
                f"{name}_reg",
            )
            if not decl.input:
                reg_to_mem.append(Enable(reg_to_mem_group(main, name, reg, mem)))

    if not arr_size:
        raise CompileError(
            "Failed to infer array size. Are there no array declarations?"
        )

    # Collect implicit memory and register declarations.
    for stmt in prog.stmts:
        if stmt.dst not in used_names:
            if isinstance(stmt.operation, ast.Map):
                name = stmt.dst
                par = par_factor[name]
                for i in range(par):
                    main.comb_mem_d1(f"{name}_b{i}", 32, arr_size // par, 32)
            else:
                raise NotImplementedError("Generating register declarations")
                #  cells.append(emit_reg_decl(stmt.dest, 32))
            used_names.append(stmt.dst)

    control: List[Control] = []
    # Generate Calyx for each statement
    for i, stmt in enumerate(prog.stmts):
        control.append(
            gen_stmt_impl(main, stmt, arr_size, par_factor, i, use_my_map_impl)
        )

    # For each output register, move the value of the register into the external array
    if reg_to_mem:
        control.append(ParComp(reg_to_mem))

    main.control = SeqComp(control)
    # Generate the Calyx program
    calyx_prog.program.emit()
