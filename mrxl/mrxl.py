import lark


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


def interp(prog):
    decls, stmts = prog.children

    for decl in decls.children:
        qual, name, typ = decl.children
        print(qual.data, str(name))


def main():
    parser = lark.Lark(GRAMMAR)
    tree = parser.parse("""
    input foo: bar
    input foo2: bar2
    baz := map 5 (5) {5}
    """)
    print(tree.pretty())
    interp(tree)


if __name__ == '__main__':
    main()
