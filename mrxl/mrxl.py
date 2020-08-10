import lark
from dataclasses import dataclass
from typing import List, Union


GRAMMAR = """
start: decls stmts
decls: decl*
stmts: stmt*

decl: qual CNAME ":" type
stmt: CNAME ":=" (map | reduce)
map: "map" INT bind block
reduce: "reduce" INT bind litexpr block
?block: "{" expr "}"
bind: "(" expr ")"

?expr: binexpr | litexpr | varexpr
binexpr: expr binop expr
litexpr: INT
varexpr: CNAME
binop: "+" -> add
     | "-" -> sub
     | "*" -> mul
     | "/" -> div

type: CNAME
qual: "input" -> input | "output" -> output

%import common.INT
%import common.WS
%import common.CNAME
%ignore WS
""".strip()


# AST classes.

@dataclass
class Decl:
    input: bool  # Otherwise, output.
    name: str
    type: str  # TODO


@dataclass
class BinExpr:
    op: str
    lhs: "Expr"
    rhs: "Expr"


@dataclass
class LitExpr:
    value: int


@dataclass
class VarExpr:
    name: str


Expr = Union[BinExpr, LitExpr, VarExpr]


@dataclass
class Map:
    par: int
    bind: str  # TODO
    body: Expr


@dataclass
class Reduce:
    par: int
    bind: str  # TODO
    init: int
    body: Expr


@dataclass
class Stmt:
    dest: str
    op: Union[Map, Reduce]


@dataclass
class Prog:
    decls: List[Decl]
    stmts: List[Stmt]


# Transform parse tree to AST.

class ConstructAST(lark.Transformer):
    def decl(self, args):
        qual, name, typ = args
        return Decl(qual.data == "input", str(name), repr(typ))

    def start(self, args):
        decls, stmts = args
        return Prog(decls.children, stmts.children)

    def stmt(self, args):
        dest, op = args
        return Stmt(str(dest), op)

    def map(self, args):
        par, bind, block = args
        return Map(int(par), str(bind), str(block))

    def reduce(self, args):
        par, bind, init, block = args
        return Map(int(par), str(bind), int(init), str(block))

    def binexpr(self, args):
        lhs, op, rhs = args
        return BinExpr(op.data, lhs, rhs)

    def litexpr(self, args):
        value, = args
        return LitExpr(int(value))

    def varexpr(self, args):
        name, = args
        return VarExpr(str(name))


# Interpreter.

def interp(prog: Prog):
    for decl in prog.decls:
        print(decl.input, decl.name)

    for stmt in prog.stmts:
        print(stmt.dest, stmt.op.body)


def main():
    parser = lark.Lark(GRAMMAR)
    tree = parser.parse("""
    input foo: bar
    output foo2: bar2
    baz := map 5 (5) { a + 5 }
    """)
    ast = ConstructAST().transform(tree)
    interp(ast)


if __name__ == '__main__':
    main()
