from typing import Dict, List, Tuple
from . import ast
from calyx.py_ast import (
    Connect,
    Group,
    CompVar,
    Stdlib,
    SeqComp,
    ConstantPort,
    HolePort,
    CompPort,
    Enable,
    While,
    ParComp,
    Control,
)
import calyx.builder as cb


def emit_compute_op(exp, op, dest, name2arr, suffix, bank_suffix):
    """
    Returns a string containing a Calyx implementation of a MrXL
    variable or number (exp). op is the type of operation this
    expression is used in. dest is the destination of this expression.
    name2arr maps statement variable names to the array names they're
    accessing elements of, e.g. if we're binding an element of array
    `foo` to a variable `a`, `a` maps to `foo`.
    """
    if isinstance(exp, ast.VarExpr):
        if isinstance(op, ast.Map):
            return CompPort(CompVar(f"{name2arr[exp.name]}{bank_suffix}"), "read_data")
        else:
            return CompPort(CompVar(f"{dest}"), "out")
    else:
        return ConstantPort(32, exp.value)


def emit_eval_body_group(s_idx, stmt: ast.Stmt, b=None):
    """
    Returns a string of a group that implements the body
    of stmt, a `map` or `reduce` statement. Adds suffix
    at the end of the group name, to avoid name collisions
    with other `map` or `reduce` statement group implementations.
    If this is a `map` expression, b is the banking factor
    of the input array. (Otherwise, b is None.)
    """
    bank_suffix = "_b" + str(b) if b is not None else ""

    mem_offsets = []
    name2arr = dict()
    for bi in stmt.op.bind:
        idx = 0 if isinstance(stmt.op, ast.Map) else 1
        name2arr[bi.dest[idx]] = bi.src
        src = CompVar(f"{bi.src}{bank_suffix}")
        dest = CompVar(f"idx{bank_suffix}_{s_idx}")

        mem_offsets.append(Connect(CompPort(src, "addr0"), CompPort(dest, "out")))

    if isinstance(stmt.op, ast.Map):
        src = CompVar(f"{stmt.dest}{bank_suffix}")
        dest = CompVar(f"idx{bank_suffix}_{s_idx}")
        mem_offsets.append(Connect(CompPort(src, "addr0"), CompPort(dest, "out")))

    compute_left_op = emit_compute_op(
        stmt.op.body.lhs, stmt.op, stmt.dest, name2arr, s_idx, bank_suffix
    )

    compute_right_op = emit_compute_op(
        stmt.op.body.rhs, stmt.op, stmt.dest, name2arr, s_idx, bank_suffix
    )

    if isinstance(stmt.op, ast.Map):
        write_to = CompVar(f"{stmt.dest}{bank_suffix}")
        adder_op = CompVar(f"adder_op{bank_suffix}_{s_idx}")
        write_connection = Connect(
            CompPort(write_to, "write_data"), CompPort(adder_op, "out")
        )
    else:
        write_connection = Connect(
            CompPort(CompVar(f"{stmt.dest}"), "in"),
            CompPort(CompVar(f"adder_op{s_idx}"), "out"),
        )
    group_id = CompVar(f"eval_body{bank_suffix}_{s_idx}")
    adder = CompVar(f"adder_op{bank_suffix}_{s_idx}")
    dest = CompVar(f"{stmt.dest}{bank_suffix}")
    return Group(
        id=group_id,
        connections=[
            Connect(CompPort(dest, "write_en"), ConstantPort(1, 1)),
            Connect(CompPort(adder, "left"), compute_left_op),
            Connect(CompPort(adder, "right"), compute_right_op),
            write_connection,
            Connect(HolePort(group_id, "done"), CompPort(dest, "done")),
        ]
        + mem_offsets,
    )


def cond_group(
    comp: cb.ComponentBuilder, idx: cb.CellBuilder, arr_size: int, suffix: str
) -> Tuple[str, str]:
    """
    Creates a group that checks if the index is less than the array size.
    """
    group_name = f"cond_{suffix}"
    stdlib = Stdlib()
    cell = f"lt_{suffix}"
    lt = comp.cell(cell, stdlib.op("lt", 32, signed=False))
    with comp.comb_group(group_name):
        lt.left = idx.out
        lt.right = cb.const(32, arr_size)

    return cell, group_name


def incr_group(comp: cb.ComponentBuilder, idx: cb.CellBuilder, suffix: str) -> str:
    """
    Creates a group that increments the index.
    """
    group_name = f"incr_idx_{suffix}"
    adder = comp.add(f"incr_{suffix}", 32)
    with comp.group(group_name) as incr:
        adder.left = idx.out
        adder.right = 1
        idx.in_ = adder.out
        idx.write_en = 1
        incr.done = idx.done

    return group_name


def gen_reduce_impl(
    comp: cb.ComponentBuilder, dest: str, stmt: ast.Reduce, arr_size: int, s_idx: int
):
    """
    Implements a `reduce` statement of the form:
        baz := reduce 5 (acc, x <- foo) init { acc + x }
    The implementation first initializes the accumulator to `init` and then directly
    accumulates the values of the array into the accumulator.
    """
    stdlib = Stdlib()

    idx = comp.reg(f"idx{s_idx}", 32)
    # Initialize the accumulator to `init`.
    init = f"init_{s_idx}"
    init_val = stmt.init
    assert isinstance(init_val, ast.LitExpr), "Reduce init must be a literal"
    with comp.group(init) as group:
        idx.in_ = init_val.value
        idx.write_en = 1
        group.done = idx.done

    # Increment the index register
    incr = incr_group(comp, idx, f"{s_idx}")
    # Check if we've reached the end of the loop
    (port, cond) = cond_group(comp, idx, arr_size, f"{s_idx}")

    # Perform the computation
    assert (
        len(stmt.bind) == 1
    ), "Reduce statements with multiple bind clauses not supported"

    # Split up the accumulator and the array element
    bind = stmt.bind[0]
    [acc, x] = bind.dest
    name2arr = {acc: (dest, "reg"), x: (f"{bind.src}_b0", "mem")}

    def expr_to_port(expr: ast.Expr):
        if isinstance(expr, ast.LitExpr):
            return cb.const(32, expr.value)
        elif isinstance(expr, ast.VarExpr):
            (bind, kind) = name2arr[expr.name]
            if kind == "mem":
                # If the mapping is defined, this is a memory
                return CompPort(CompVar(f"{bind}"), "read_data")
            else:
                # Otherwise this is a cell
                return CompPort(CompVar(f"{bind}"), "out")
        elif isinstance(expr, ast.BinExpr):
            raise NotImplementedError("Nested expressions not supported")

    if stmt.body.op == "mul":
        op = comp.cell(f"mul_{s_idx}", stdlib.op("mult_pipe", 32, signed=False))
    else:
        op = comp.add(f"add_{s_idx}", 32)
    with comp.group(f"reduce{s_idx}") as ev:
        out = comp.get_cell(dest)  # The accumulator is a register

        # The source must be a singly-banked array
        inp = comp.get_cell(f"{bind.src}_b0")
        inp.addr0 = idx.out
        op.left = expr_to_port(stmt.body.lhs)
        op.right = expr_to_port(stmt.body.rhs)
        out.in_ = op.out
        # Multipliers are sequential so we need to manipulate go/done signals
        if stmt.body.op == "mul":
            op.go = 1
            out.write_en = op.done
        else:
            out.write_en = 1
        ev.done = out.done

    control = SeqComp(
        [
            Enable(init),
            While(
                port=CompPort(CompVar(port), "out"),
                cond=CompVar(cond),
                body=SeqComp([Enable(f"reduce{s_idx}"), Enable(incr)]),
            ),
        ]
    )

    return control


def gen_map_impl(
    comp: cb.ComponentBuilder,
    dest: str,
    stmt: ast.Map,
    arr_size: int,
    bank_factor: int,
    s_idx: int,
):
    """
    Returns a dictionary containing Calyx cells, wires and
    control needed to implement a map statement. (See gen_stmt_impl
    for format of the dictionary.)

    Generates these groups:
      - a group that implements the body of the map statement
      - a group that increments an index to access the map input array
      - a group that implements the loop condition, checking if the index
        has reached the end of the input array
    """
    stdlib = Stdlib()

    # Parallel loops representing the map body
    map_loops = []

    arr_size = arr_size // bank_factor
    for bank in range(bank_factor):
        suffix = f"b{bank}_{s_idx}"
        idx = comp.reg(f"idx_{suffix}", 32)

        # Increment the index
        incr = incr_group(comp, idx, suffix)
        # Check if we've reached the end of the loop
        (port, cond) = cond_group(comp, idx, arr_size, suffix)

        # Perform the computation
        body = stmt.body
        if isinstance(body, ast.LitExpr):  # Body is a constant
            raise NotImplementedError()
        elif isinstance(body, ast.VarExpr):  # Body is a variable
            raise NotImplementedError()

        # Mapping from binding to arrays
        name2arr = {bind.dest[0]: f"{bind.src}_b{bank}" for bind in stmt.bind}

        def expr_to_port(expr: ast.Expr):
            if isinstance(expr, ast.LitExpr):
                return cb.const(32, expr.value)
            elif isinstance(expr, ast.VarExpr):
                return CompPort(CompVar(f"{name2arr[expr.name]}"), "read_data")
            elif isinstance(expr, ast.BinExpr):
                raise NotImplementedError("Nested expressions not supported")

        if body.op == "mul":
            op = comp.cell(f"mul_{suffix}", stdlib.op("mult_pipe", 32, signed=False))
        else:
            op = comp.add(f"add_{suffix}", 32)

        with comp.group(f"eval_body_{suffix}") as ev:
            assert (
                len(stmt.bind) <= 2
            ), "Map statements with more than 2 arguments not supported"
            # Index each array
            for bind in stmt.bind:
                # Map bindings have exactly one dest
                mem = comp.get_cell(f"{name2arr[bind.dest[0]]}")
                mem.addr0 = idx.out
            out_mem = comp.get_cell(f"{dest}_b{bank}")
            out_mem.addr0 = idx.out
            # Provide inputs to the op
            op.left = expr_to_port(body.lhs)
            op.right = expr_to_port(body.rhs)
            out_mem.write_data = op.out
            # Multipliers are sequential so we need to manipulate go/done signals
            if body.op == "mul":
                op.go = 1
                out_mem.write_en = op.done
            else:
                out_mem.write_en = 1
            ev.done = out_mem.done

        # Control to execute the groups
        map_loops.append(
            While(
                CompPort(CompVar(port), "out"),
                CompVar(cond),
                SeqComp(
                    [
                        Enable(f"eval_body_{suffix}"),
                        Enable(incr),
                    ]
                ),
            )
        )

    control = ParComp(map_loops)

    return control


def gen_stmt_impl(
    comp: cb.ComponentBuilder,
    stmt: ast.Stmt,
    arr_size: int,
    name2par: Dict[str, int],
    statement_idx: int,
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
    if isinstance(stmt.op, ast.Map):
        return gen_map_impl(
            comp, stmt.dest, stmt.op, arr_size, name2par[stmt.dest], statement_idx
        )
    else:
        return gen_reduce_impl(comp, stmt.dest, stmt.op, arr_size, statement_idx)


def compute_par_factors(stmts: List[ast.Stmt]) -> Dict[str, int]:
    """Maps the name of memories to their banking factors."""
    out: Dict[str, int] = dict()

    def add_par(mem: str, par: int):
        # If we've already inferred a banking factor for this memory,
        # make sure it's the same as the one we're inferring now.
        if mem in out and par != out[mem]:
            raise Exception(
                f"Previous use of `{mem}` had banking factor {out[mem]}"
                " but current use has banking factor {par}"
            )
        out[mem] = par

    for stmt in stmts:
        par_f = stmt.op.par
        if isinstance(stmt.op, ast.Reduce):
            # Reduction does not support parallelism
            if par_f != 1:
                raise Exception("Reduction does not support parallelism")
        add_par(stmt.dest, par_f)
        for b in stmt.op.bind:
            add_par(b.src, par_f)

    return out


def emit(prog: ast.Prog):
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
    # ANCHOR: collect-decls
    for decl in prog.decls:
        used_names.append(decl.name)
        if decl.type.size:  # A memory
            arr_size = decl.type.size
            name = decl.name
            par = par_factor[name]
            for i in range(par):
                main.mem_d1(f"{name}_b{i}", 32, arr_size // par, 32, is_external=True)
        else:  # A register
            main.reg(decl.name, 32)
    # ANCHOR_END: collect-decls

    if not arr_size:
        raise Exception("Failed to infer array size. Are there no array declarations?")

    # Collect implicit memory and register declarations.
    for stmt in prog.stmts:
        if stmt.dest not in used_names:
            if isinstance(stmt.op, ast.Map):
                name = stmt.dest
                par = par_factor[name]
                for i in range(par):
                    main.mem_d1(f"{name}_b{i}", 32, arr_size // par, 32)
            else:
                raise NotImplementedError("Generating register declarations")
                #  cells.append(emit_reg_decl(stmt.dest, 32))
            used_names.append(stmt.dest)

    control: List[Control] = []
    # Generate Calyx for each statement
    for i, stmt in enumerate(prog.stmts):
        control.append(gen_stmt_impl(main, stmt, arr_size, par_factor, i))

    main.control = SeqComp(control)
    # Generate the Calyx program
    calyx_prog.program.emit()
