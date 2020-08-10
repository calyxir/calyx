import lark
from . import ast


GRAMMAR = """
start: decls stmts
decls: decl*
stmts: stmt*

decl: qual CNAME ":" type
stmt: CNAME ":=" (map | reduce)
map: "map" INT binding block
reduce: "reduce" INT binding litexpr block
?block: "{" expr "}"
?binding: "(" bindlist ")"

?expr: binexpr | litexpr | varexpr
binexpr: expr binop expr
litexpr: INT
varexpr: CNAME
binop: "+" -> add
     | "-" -> sub
     | "*" -> mul
     | "/" -> div

bindlist: (bind ("," bind)*)?
bind: varlist "<-" CNAME
varlist: (CNAME ("," CNAME)*)?

type: basetype ("[" INT "]")?
basetype: "float" -> float
        | "int"   -> int

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
        return ast.Map(int(par), bind.children, block)

    def reduce(self, args):
        par, bind, init, block = args
        return ast.Reduce(int(par), bind.children, init, block)

    def binexpr(self, args):
        lhs, op, rhs = args
        return ast.BinExpr(op.data, lhs, rhs)

    def litexpr(self, args):
        value, = args
        return ast.LitExpr(int(value))

    def varexpr(self, args):
        name, = args
        return ast.VarExpr(str(name))

    def type(self, args):
        base = str(args[0])
        if len(args) == 2:
            size = int(args[1])
        else:
            size = None
        return ast.Type(base, size)

    def bind(self, args):
        dest, src = args
        return ast.Bind([str(d) for d in dest.children], str(src))


def parse(txt: str) -> ast.Prog:
    parser = lark.Lark(GRAMMAR)
    tree = parser.parse(txt)
    ast = ConstructAST().transform(tree)
    return ast
