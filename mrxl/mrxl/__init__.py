from .parse import parse
from . import ast
import sys


# Interpreter.

def interp(prog: ast.Prog):
    for decl in prog.decls:
        print(decl.input, decl.name)

    for stmt in prog.stmts:
        print(stmt.dest, stmt.op.body)


def main():
    with open(sys.argv[1]) as f:
        txt = f.read()
    ast = parse(txt)
    interp(ast)


if __name__ == '__main__':
    main()
