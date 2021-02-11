from . import ast
from futil.ast import *


def emit_mem_decl(name, size, par):
    """
    Returns N memory declarations,
    where N = `par`.
    """
    stdlib = Stdlib()
    banked_mems = []
    for i in range(par):
        banked_mems.append(
            Cell(CompVar(f"{name}_b{i}"), stdlib.mem_d1(32, size // par, 32))
        )
    return banked_mems


def emit_cond_group(suffix, arr_size, b=None):
    """
    Emits a group that checks if an index has reached
    arr_size. If the bank number `b` is not None, adds it
    to the end of the index cell name.

    suffix is added to the end to the end of each cell,
    to disambiguate from other `map` or `reduce` implementations.
    """
    bank_suffix = f"_b{b}_" if b is not None else ""
    group_id = CompVar(f"cond{bank_suffix}{suffix}")
    le = CompVar(f"le{bank_suffix}{suffix}")
    idx = CompVar(f"idx{bank_suffix}{suffix}")
    return Group(
        id=group_id,
        connections=[
            Connect(CompPort(idx, "out"), CompPort(le, "left")),
            Connect(ConstantPort(32, arr_size), CompPort(le, "right")),
            Connect(ConstantPort(1, 1), HolePort(group_id, "done")),
        ],
    )


def emit_idx_group(s_idx, b=None):
    """
    Emits a group that increments an index.
    If the bank number `b` is not None, adds
    it (the bank number) as a suffix to each
    cell name.
    """
    bank_suffix = "_b" + str(b) + "_" if b is not None else ""
    group_id = CompVar(f"incr_idx{bank_suffix}{s_idx}")
    adder = CompVar(f"adder_idx{bank_suffix}{s_idx}")
    idx = CompVar(f"idx{bank_suffix}{s_idx}")
    return Group(
        id=group_id,
        connections=[
            Connect(CompPort(idx, "out"), CompPort(adder, "left")),
            Connect(ConstantPort(32, 1), CompPort(adder, "right")),
            Connect(ConstantPort(1, 1), CompPort(idx, "write_en")),
            Connect(CompPort(adder, "out"), CompPort(idx, "in")),
            Connect(CompPort(idx, "done"), HolePort(group_id, "done")),
        ],
    )


def emit_compute_op(exp, op, dest, name2arr, suffix, bank_suffix):
    """
    Returns a string containing a FuTIL implementation of a MrXL
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


def emit_eval_body_group(s_idx, stmt, b=None):
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

        mem_offsets.append(Connect(CompPort(dest, "out"), CompPort(src, "addr0")))

    if isinstance(stmt.op, ast.Map):
        src = CompVar(f"{stmt.dest}{bank_suffix}")
        dest = CompVar(f"idx{bank_suffix}_{s_idx}")
        mem_offsets.append(Connect(CompPort(dest, "out"), CompPort(src, "addr0")))

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
            CompPort(adder_op, "out"), CompPort(write_to, "write_data")
        )
    else:
        write_connection = Connect(
            CompPort(CompVar(f"adder_op{s_idx}"), "out"),
            CompPort(CompVar(f"{stmt.dest}"), "in"),
        )
    group_id = CompVar(f"eval_body{bank_suffix}_{s_idx}")
    adder = CompVar(f"adder_op{bank_suffix}_{s_idx}")
    dest = CompVar(f"{stmt.dest}{bank_suffix}")
    return Group(
        id=group_id,
        connections=[
            Connect(ConstantPort(1, 1), CompPort(dest, "write_en")),
            Connect(compute_left_op, CompPort(adder, "left")),
            Connect(compute_right_op, CompPort(adder, "right")),
            write_connection,
            Connect(CompPort(dest, "done"), HolePort(group_id, "done")),
        ]
        + mem_offsets,
    )


def gen_reduce_impl(stmt, arr_size, s_idx):
    """
    Returns a dictionary containing FuTIL cells, wires and
    control needed to implement a map statement. Similar
    to gen_map_impl, with an implementation of a body
    of the `reduce` statement instead of an implementation
    of a `map` statement.
    """
    result = dict()
    stdlib = Stdlib()
    op_name = "mult" if stmt.op.body.op == "mul" else "add"
    cells = [
        Cell(CompVar(f"le{s_idx}"), stdlib.op("lt", 32, signed=False)),
        Cell(CompVar(f"idx{s_idx}"), stdlib.register(32)),
        Cell(CompVar(f"adder_idx{s_idx}"), stdlib.op("add", 32, signed=False)),
        Cell(CompVar(f"adder_op{s_idx}"), stdlib.op(f"{op_name}", 32, signed=False)),
    ]
    wires = [
        emit_cond_group(s_idx, arr_size),
        emit_idx_group(s_idx),
        emit_eval_body_group(s_idx, stmt, 0),
    ]
    control = While(
        port=CompPort(CompVar(f"le{s_idx}"), "out"),
        cond=CompVar(f"cond{s_idx}"),
        body=SeqComp([Enable(f"eval_body{s_idx}"), Enable(f"incr_idx{s_idx}")]),
    )

    return {"cells": cells, "wires": wires, "control": control}


def gen_map_impl(stmt, arr_size, bank_factor, s_idx):
    """
    Returns a dictionary containing FuTIL cells, wires and
    control needed to implement a map statement. (See gen_stmt_impl
    for format of the dictionary.)

    Generates these groups:
      - a group that implements the body of the map statement
      - a group that increments an index to access the map input array
      - a group that implements the loop condition, checking if the index
        has reached the end of the input array
    """
    result = dict()
    stdlib = Stdlib()

    cells = []
    for b in range(bank_factor):
        cells.extend(
            [
                Cell(CompVar(f"le_b{b}_{s_idx}"), stdlib.op("lt", 32, signed=False)),
                Cell(CompVar(f"idx_b{b}_{s_idx}"), stdlib.register(32)),
                Cell(
                    CompVar(f"adder_idx_b{b}_{s_idx}"),
                    stdlib.op("add", 32, signed=False),
                ),
            ]
        )

    op_name = "mult" if stmt.op.body.op == "mul" else "add"
    for b in range(bank_factor):
        cells.append(
            Cell(
                CompVar(f"adder_op_b{b}_{s_idx}"),
                stdlib.op(f"{op_name}", 32, signed=False),
            )
        )

    wires = []
    for b in range(bank_factor):
        wires.extend(
            [
                emit_cond_group(s_idx, arr_size // bank_factor, b),
                emit_idx_group(s_idx, b),
                emit_eval_body_group(s_idx, stmt, b),
            ]
        )

        map_loops = []
        for b in range(bank_factor):
            b_suffix = f"_b{str(b)}_"
            map_loops.append(
                While(
                    CompPort(CompVar(f"le{b_suffix}{s_idx}"), "out"),
                    CompVar(f"cond{b_suffix}{s_idx}"),
                    SeqComp(
                        [
                            Enable(f"eval_body{b_suffix}{s_idx}"),
                            Enable(f"incr_idx{b_suffix}{s_idx}"),
                        ]
                    ),
                )
            )

    control = ParComp(map_loops)

    return {"cells": cells, "wires": wires, "control": control}


def gen_stmt_impl(stmt, arr_size, name2par, s_idx):
    """
    Returns FuTIL cells, wires, and control needed to implement
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
        return gen_map_impl(stmt, arr_size, name2par[stmt.dest], s_idx)
    else:
        return gen_reduce_impl(stmt, arr_size, s_idx)


def emit(prog):
    """
    Returns a string containing a FuTIL program, compiled from `prog`, a MrXL
    program.
    """
    cells, wires, control = [], [], []

    # All arrays must be the same size. The first array we see determines the
    # size that we'll assume for the rest of the program's arrays.
    arr_size = None

    # Collect banking factors.
    name2par = dict()
    for stmt in prog.stmts:
        if isinstance(stmt.op, ast.Map):
            name2par[stmt.dest] = stmt.op.par
            for b in stmt.op.bind:
                name2par[b.src] = stmt.op.par

    # Collect memory and register declarations.
    used_names = []
    stdlib = Stdlib()
    for decl in prog.decls:
        used_names.append(decl.name)
        if decl.type.size:  # A memory
            arr_size = decl.type.size
            cells.extend(emit_mem_decl(decl.name, decl.type.size, name2par[decl.name]))
        else:  # A register
            cells.append(Cell(CompVar(decl.name), stdlib.register(32)))

    # Collect implicit memory and register declarations.
    for stmt in prog.stmts:
        if stmt.dest not in used_names:
            if isinstance(stmt.op, ast.Map):
                cells.extend(emit_mem_decl(stmt.dest, arr_size, name2par[stmt.dest]))
            else:
                cells.append(emit_reg_decl(stmt.dest, 32))
            used_names.append(stmt.dest)

    # Generate FuTIL.
    for i, stmt in enumerate(prog.stmts):
        stmt_impl = gen_stmt_impl(stmt, arr_size, name2par, i)
        cells.extend(stmt_impl["cells"])
        wires.extend(stmt_impl["wires"])
        control.append(stmt_impl["control"])

    program = Program(
        imports=[Import("primitives/std.lib")],
        components=[
            Component(
                name="main",
                inputs=[],
                outputs=[],
                structs=cells + wires,
                controls=SeqComp(control),
            )
        ],
    )
    program.emit()
