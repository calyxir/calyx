from . import ast
from functools import reduce


class InterpError(Exception):
    """Interpretation failed unrecoverably.
    """


def _dict_zip(d):
    """Given a dict of lists, generate a sequence of dicts with the same
    keys---each associated with one "slice" of the lists.
    """
    for i in range(len(next(iter(d.values())))):
        yield {k: v[i] for k, v in d.items()}


def interp_expr(expr: ast.Expr, env):
    """Interpret a MrXL expression to a scalar value.
    """
    if isinstance(expr, ast.LitExpr):
        return expr.value
    elif isinstance(expr, ast.VarExpr):
        return env[expr.name]
    elif isinstance(expr, ast.BinExpr):
        lhs = interp_expr(expr.lhs, env)
        rhs = interp_expr(expr.rhs, env)
        if expr.op == "add":
            return lhs + rhs
        elif expr.op == "mul":
            return lhs * rhs
        elif expr.op == "sub":
            return lhs - rhs
        elif expr.op == "div":
            return lhs / rhs
        else:
            raise InterpError(f"unhandled binary operator: {expr.op}")
    else:
        raise InterpError(f"unhandled expression: {type(expr)}")


def interp(prog: ast.Prog, data):
    """Interpret a MrXL program, starting with some values for the input
    variables and producing some values for the output variables.
    """
    env = {}

    # Load input data into environment.
    for decl in prog.decls:
        if decl.input:
            try:
                env[decl.name] = data[decl.name]
            except KeyError:
                raise InterpError(f"input data for `{decl.name}` not found")

    for stmt in prog.stmts:
        if isinstance(stmt.op, ast.Map):
            map_data = {}
            for bind in stmt.op.bind:
                if len(bind.dest) != 1:
                    raise InterpError("map binds are unary")
                try:
                    map_data[bind.dest[0]] = env[bind.src]
                except KeyError:
                    raise InterpError(f"source `{bind.src}` for map not found")

            # Compute the map.
            env[stmt.dest] = [
                interp_expr(stmt.op.body, env)
                for env in _dict_zip(map_data)
            ]

        elif isinstance(stmt.op, ast.Reduce):
            if len(stmt.op.bind) != 1:
                raise InterpError("reduce needs only one bind")
            bind = stmt.op.bind[0]
            if len(bind.dest) != 2:
                raise InterpError("reduce requires a binary bind")

            try:
                red_data = env[bind.src]
            except KeyError:
                raise InterpError(f"source `{bind.src}` for reduce not found")

            init = interp_expr(stmt.op.init, {})

            # Compute the reduce.
            env[stmt.dest] = reduce(
                lambda x, y: interp_expr(
                    stmt.op.body,
                    {bind.dest[0]: x, bind.dest[1]: y},
                ),
                red_data,
                init,
            )

        else:
            raise InterpError(f"unknown op {type(stmt.op)}")

    # Emit the output values.
    out = {}
    for decl in prog.decls:
        if not decl.input:
            try:
                out[decl.name] = env[decl.name]
            except KeyError:
                raise InterpError(f"output value `{decl.name}` not found")
    return out
