import lark
from . import ast


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


# Transform parse tree to AST.

class ConstructAST(lark.Transformer):
    def decl(self, args):
        qual, name, typ = args
        return ast.Decl(qual.data == "input", str(name), repr(typ))

    def start(self, args):
        decls, stmts = args
        return ast.Prog(decls.children, stmts.children)

    def stmt(self, args):
        dest, op = args
        return ast.Stmt(str(dest), op)

    def map(self, args):
        par, bind, block = args
        return ast.Map(int(par), str(bind), str(block))

    def reduce(self, args):
        par, bind, init, block = args
        return ast.Map(int(par), str(bind), int(init), str(block))

    def binexpr(self, args):
        lhs, op, rhs = args
        return ast.BinExpr(op.data, lhs, rhs)

    def litexpr(self, args):
        value, = args
        return ast.LitExpr(int(value))

    def varexpr(self, args):
        name, = args
        return ast.VarExpr(str(name))


def parse(txt: str) -> ast.Prog:
    parser = lark.Lark(GRAMMAR)
    tree = parser.parse(txt)
    ast = ConstructAST().transform(tree)
    return ast
