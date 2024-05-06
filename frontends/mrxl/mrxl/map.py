from calyx.py_ast import (
    CompVar,
    Stdlib,
    SeqComp,
    CompPort,
    Enable,
    While,
    ParComp,
)
from . import ast
import calyx.builder as cb


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
    control needed to implement a `map` statement.
    See gen_stmt_impl for format of the dictionary.

    Generates these groups:
      - a group that implements the body of the `map` statement
      - a group that increments an index to access the `map` input array
      - a group that implements the loop condition, checking if the index
        has reached the end of the input array
    """
    from .gen_calyx import incr_group, cond_group, CompileError

    # Parallel loops representing the `map` body
    map_loops = []

    arr_size = arr_size // bank_factor
    for bank in range(bank_factor):
        suffix = f"b{bank}_{s_idx}"
        idx = comp.reg(32, f"idx_{suffix}")

        # Increment the index
        incr = incr_group(comp, idx, suffix)
        # Check if we've reached the end of the loop
        (port, cond) = cond_group(comp, idx, arr_size, suffix)

        # Perform the computation
        body = stmt.body
        if isinstance(body, ast.LitExpr):  # Body is a constant
            raise NotImplementedError()
        if isinstance(body, ast.VarExpr):  # Body is a variable
            raise NotImplementedError()

        # Mapping from binding to arrays
        name2arr = {bind.dst[0]: f"{bind.src}_b{bank}" for bind in stmt.binds}

        def expr_to_port(expr: ast.BaseExpr):
            if isinstance(expr, ast.LitExpr):
                return cb.const(32, expr.value)
            if isinstance(expr, ast.VarExpr):
                return CompPort(CompVar(name2arr[expr.name]), "read_data")
            raise CompileError(f"Unhandled expression: {type(expr)}")

        # ANCHOR: map_op
        if body.operation == "mul":
            operation = comp.cell(
                f"mul_{suffix}", Stdlib.op("mult_pipe", 32, signed=False)
            )
        else:
            operation = comp.add(32, f"add_{suffix}")
        # ANCHOR_END: map_op

        assert (
            len(stmt.binds) <= 2
        ), "Map statements with more than 2 arguments not supported"
        # ANCHOR: map_inputs
        with comp.group(f"eval_body_{suffix}") as evl:
            # Index each array
            for bind in stmt.binds:
                # Map bindings have exactly one dest
                mem = comp.get_cell(f"{name2arr[bind.dst[0]]}")
                mem.addr0 = idx.out
            # ANCHOR_END: map_inputs
            # Provide inputs to the op
            operation.left = expr_to_port(body.lhs)
            operation.right = expr_to_port(body.rhs)
            # ANCHOR: map_write
            out_mem = comp.get_cell(f"{dest}_b{bank}")
            out_mem.addr0 = idx.out
            out_mem.write_data = operation.out
            # Multipliers are sequential so we need to manipulate go/done signals
            if body.operation == "mul":
                operation.go = 1
                out_mem.write_en = operation.done
            else:
                out_mem.write_en = 1
            evl.done = out_mem.done
            # ANCHOR_END: map_write

        # Control to execute the groups
        map_loops.append(
            # ANCHOR: map_loop
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
            # ANCHOR_END: map_loop
        )

    control = ParComp(map_loops)

    return control
