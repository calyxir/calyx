from tvm import relay
from tvm.relay.expr_functor import ExprFunctor


class ToSource(ExprFunctor):
    def __init__(self):
        super(ToSource, self).__init__()
        self.declare = ""
        self.input_const = []

    def visit_var(self, var):
        return "a variable called {}".format(var.name_hint)

    def visit_function(self, func):
        expr = self.visit(func.body)

        source = self.declare

        args = ""
        if isinstance(func, relay.GlobalVar):
            func = self.gv_map[func]
        end = len(func.params) - 1
        init = ""
        for i in range(len(func.params)):
            args += f"args[{i+len(self.input_const)}]"
            if i != end:
                args += ", "

        source += f"""
        args: {args}
        init: {init}
        function: {expr}
        }});
        """
        return source


def to_source(mod, program, gv_map, ctx, name) -> str:
    convert = ToSource()
    return convert.visit(program)
