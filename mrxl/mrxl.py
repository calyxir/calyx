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
reduce: "reduce" INT bind literal block
block: "{" expr "}"
bind: "(" expr ")"

expr: literal
literal: INT
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
class Map:
    par: int
    bind: str  # TODO
    body: str  # TODO expr


@dataclass
class Reduce:
    par: int
    bind: str  # TODO
    init: int
    body: str  # TODO expr


@dataclass
class Stmt:
    dest: str
    op: Union[Map, Reduce]


@dataclass
class Prog:
    decls: List[Decl]
    stmts: List[str]


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


def interp(prog: Prog):
    for decl in prog.decls:
        print(decl.input, decl.name)


def main():
    parser = lark.Lark(GRAMMAR)
    tree = parser.parse("""
    input foo: bar
    output foo2: bar2
    baz := map 5 (5) {5}
    """)
    ast = ConstructAST().transform(tree)
    interp(ast)


if __name__ == '__main__':
    main()
