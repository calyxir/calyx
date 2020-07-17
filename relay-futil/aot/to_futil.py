from . import futil
from tvm import relay


class ExprWithStmt:
    def __init__(self, expr, stmt=""):
        assert isinstance(expr, str)
        assert isinstance(stmt, str)
        assert "ExprWithStmt" not in expr
        assert "ExprWithStmt" not in stmt
        self.expr = expr
        self.stmt = stmt

    def __str__(self):
        return f"ExprWithStmt({self.expr}, {self.stmt})"

    def __repr__(self):
        return self.__str__()


class ToSource:
    def __init__(self, gv_map):
        self.gv_map = gv_map
        self.name_counter = 0
        self.source_content = ""
        self.name_map = {}
        self.local = True
        self.declare = ""
        self.declare_map = {}
        self.input_const = []

    def fresh_global_name(self):
        name = f"global{self.name_counter}"
        self.name_counter += 1
        return name

    def sanitize(self, str):
        return str.replace("-", "_").replace("/", "_")

    def fresh_local_name(self, var=None):
        if var is not None:
            name = f"local_{self.sanitize(var.name_hint)}_{self.name_counter}"
        else:
            name = f"local_{self.name_counter}"
        self.name_counter += 1
        return name

    def fresh_label_name(self):
        name = f"label_{self.name_counter}"
        self.name_counter += 1
        return name

    # return (str, str) with lhs being stmts, and rhs being expression
    def visit(self, node, *, local=True, name=None):
        if isinstance(node, futil.FutilFunc):
            res = self.visit_futilFunc(node)
        else:
            # raise Exception(str(node))
            res = ExprWithStmt("dummy", "")
        assert isinstance(res, ExprWithStmt)
        return res

    def visit_futilFunc(self, func):
        return ExprWithStmt("function", "")

    def mk_register_api(self, name: str, func) -> str:
        vf = self.visit(func, local=False)
        assert vf.stmt == ""
        source = self.declare

        args = ""
        if isinstance(func, relay.GlobalVar):
            func = self.gv_map[func]
        end = len(func.params) - 1
        init = ""
        for i, (input_name, _) in enumerate(self.input_const):
            init += f"{input_name} = args[{i}];\n"
        for i in range(len(func.params)):
            args += f"args[{i+len(self.input_const)}]"
            if i != end:
                args += ", "

        source += f"""
        args: {args}
        init: {init}
        function: {vf.expr}
        }});
        """
        return source


def mk_file(body, ctx):
    return f"""
    futil function
    {body}
    """


def to_source(mod, program, gv_map, ctx, name) -> str:
    convert = ToSource(gv_map)
    ret = mk_file(convert.mk_register_api(name, program), ctx)
    # print([value for name, value in convert.input_const])
    # return [], "hello"
    return ret
