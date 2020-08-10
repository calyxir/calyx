from .parse import parse
from . import ast


# Interpreter.

def interp(prog: ast.Prog):
    for decl in prog.decls:
        print(decl.input, decl.name)

    for stmt in prog.stmts:
        print(stmt.dest, stmt.op.body)


def main():
    ast = parse("""
    input foo: bar
    output foo2: bar2
    baz := map 5 (5) { a + 5 }
    """)
    interp(ast)


if __name__ == '__main__':
    main()
