import tvm
from tvm import relay
from tvm.relay.function import Function
from tvm.relay.expr_functor import ExprFunctor
from tvm.relay.backend import compile_engine
from .futil import FutilFunc
from . import to_futil


def is_primitive(e: relay.Expr):
    return isinstance(e, relay.Function) and e.attrs and \
        e.attrs.Primitive.value == 1


class AoTCompiler(ExprFunctor):
    def __init__(self, mod, tgt) -> None:
        super().__init__()
        self.mod = mod
        self.tgt = tgt
        self.engine = compile_engine.get()
        self.bindings = [[]]
        self.gv_map = {}

    def add_binding(self, var, value):
        pass

    def optimize(self, expr: Function) -> Function:
        opts = tvm.transform.Sequential([relay.transform.FuseOps(),
                                         relay.transform.ToANormalForm()])
        self.mod['main'] = expr
        self.mod = opts(self.mod)
        ret = self.mod['main']
        return ret

    def visit_function(self, func):
        if is_primitive(func):
            body = self.mk_primitive_op(func, func.params, func.ret_type)
            return FutilFunc(func.params, body, func.checked_type.ret_type)
        else:
            return FutilFunc(func.params, self.visit(func.body),
                             func.checked_type.ret_type)

    def visit_constant(self, const):
        return const

    def visit_var(self, var):
        return var


def compile(func, mod, ctx, tgt, name='default'):
    assert isinstance(func, Function)
    compiler = AoTCompiler(mod, tgt)
    func = compiler.optimize(func)
    func = compiler.visit(func)
    source_code = to_futil.to_source(mod, func, compiler.gv_map, ctx, name)
    return source_code
