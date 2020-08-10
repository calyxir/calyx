from .parse import parse
from . import ast
import sys
import json


class InterpError(Exception):
    pass


def interp(prog: ast.Prog, data):
    env = {}

    # Load input data into environment.
    for decl in prog.decls:
        if decl.input:
            if decl.name not in data:
                raise InterpError(f"data for `{decl.name}` not found")
            env[decl.name] = data[decl.name]

    for stmt in prog.stmts:
        print(stmt.dest, stmt.op.body)


def main():
    with open(sys.argv[1]) as f:
        txt = f.read()
    with open(sys.argv[2]) as f:
        data = json.load(f)
    ast = parse(txt)

    try:
        interp(ast, data)
    except InterpError as exc:
        print(str(exc), file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
