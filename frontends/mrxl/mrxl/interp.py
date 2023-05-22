from functools import reduce
from typing import List, Dict, Union
from . import ast


Scalar = Union[float, int]
Array = Union[List[float], List[int]]
Value = Union[Scalar, Array]
Env = Dict[str, Value]
ScalarEnv = Dict[str, Scalar]


class InterpError(Exception):
    """Interpretation failed; recovery was impossible."""


def _dict_zip(dicts):
    """Given a dict of lists, generate a sequence of dicts with the same
    keys---each associated with one "vertical slice" of the lists.
    """
    for i in range(len(next(iter(dicts.values())))):
        yield {k: v[i] for k, v in dicts.items()}


def interp_expr(expr, env: ScalarEnv) -> Scalar:
    """Resolve a MrXL expression to a scalar value, and return it."""
    if isinstance(expr, ast.LitExpr):
        return expr.value
    if isinstance(expr, ast.VarExpr):
        return env[expr.name]
    if isinstance(expr, ast.BinExpr):
        lhs = interp_expr(expr.lhs, env)
        rhs = interp_expr(expr.rhs, env)
        if expr.operation == "add":
            return lhs + rhs
        if expr.operation == "mul":
            return lhs * rhs
        if expr.operation == "sub":
            return lhs - rhs
        if expr.operation == "div":
            return lhs / rhs
        raise InterpError(f"Unhandled binary operator: {expr.operation}")
    raise InterpError(f"Unhandled expression: {type(expr)}")


def interp_map(operation: ast.Map, env: Env) -> Array:
    """Run a `map` operation and return the resultant array."""
    map_data = {}
    for bind in operation.binds:
        if len(bind.dst) != 1:
            raise InterpError("`map` binds must be unary")
        try:
            map_data[bind.dst[0]] = env[bind.src]
        except KeyError as exc:
            raise InterpError(f"Source `{bind.src}` for `map` not found") from exc
    # Compute the map.
    return [interp_expr(operation.body, env) for env in _dict_zip(map_data)]


def interp_reduce(operation: ast.Reduce, env: Env) -> Scalar:
    """Run a `reduce` operation and return the resultant scalar."""
    if len(operation.binds) != 1:
        raise InterpError("`reduce` needs only one bind")
    bind = operation.binds[0]
    if len(bind.dst) != 2:
        raise InterpError("`reduce` requires a binary bind")

    try:
        red_data = env[bind.src]
    except KeyError as exc:
        raise InterpError(f"Source `{bind.src}` for `reduce` not found") from exc
    if not isinstance(red_data, list):
        raise InterpError("The data passed to `reduce` must be in an array")

    init = interp_expr(operation.init, {})

    # Compute the reduce.
    return reduce(
        lambda x, y: interp_expr(
            operation.body,
            {bind.dst[0]: x, bind.dst[1]: y},
        ),
        red_data,
        init,
    )


def interp(prog: ast.Prog, data: Env) -> Env:
    """Interpret a MrXL program, starting with some values for the input
    variables and producing some values for the output variables.
    """
    env = {}

    # Load input data into environment.
    for decl in prog.decls:
        if decl.input:
            try:
                env[decl.name] = data[decl.name]
            except KeyError as exc:
                raise InterpError(f"input data for `{decl.name}` not found") from exc

    # Run the program.
    for stmt in prog.stmts:
        if isinstance(stmt.operation, ast.Map):
            env[stmt.dst] = interp_map(stmt.operation, env)
        elif isinstance(stmt.operation, ast.Reduce):
            env[stmt.dst] = interp_reduce(stmt.operation, env)
        else:
            raise InterpError(f"Unknown operation {type(stmt.operation)}")

    # Emit the output values.
    out = {}
    for decl in prog.decls:
        if not decl.input:
            try:
                out[decl.name] = env[decl.name]
            except KeyError as exc:
                raise InterpError(f"Output value `{decl.name}` not found") from exc
    return out
