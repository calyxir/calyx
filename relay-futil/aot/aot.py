import ctypes
import numpy as np
import os
import subprocess
import tempfile
import tvm
from tvm import relay, get_global_func, target, register_func
from tvm.relay.function import Function
from tvm.relay.expr import Expr, Let, GlobalVar
from tvm.relay.adt import Constructor
from tvm.relay.expr_functor import ExprFunctor, ExprVisitor
from tvm.relay.backend import compile_engine
from .futil import FutilFunc
from . import to_futil
from .convert import convert

def must_run_process(args):
    proc = subprocess.run(args)
    assert proc.returncode == 0

def load_lib(name):
    return ctypes.CDLL(name, ctypes.RTLD_GLOBAL)

def is_primitive(e: relay.Expr):
    return isinstance(e, relay.Function) and e.attrs and e.attrs.Primitive.value == 1

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
            return FutilFunc(func.params, self.visit(func.body), func.checked_type.ret_type)

    def visit_constant(self, const):
        return const
    
    def visit_var(self, var):
        return var

_LIB_COUNTER = 1
_LIB = []

def lib_and_func_name(name):
    global _LIB_COUNTER
    packed_name = f'relay.aot.{name}.{_LIB_COUNTER}'
    lib_name = f"librelay_aot_{_LIB_COUNTER}.so"
    _LIB_COUNTER += 1
    return lib_name, packed_name

import time

def _mk_wrapper(fn, ctx, constants, record_time):
    def _wrapper(*args):
        new_constants = [convert(a, ctx) for a in constants]
        new_args = [convert(a, ctx) for a in args]
        begin = time.perf_counter()
        res = fn(*new_constants, *new_args)
        end = time.perf_counter()
        return res if not record_time else (res, end - begin)
    return _wrapper

import sys
sys.setrecursionlimit(10000)

def compile(func, mod, ctx, tgt, name='default', record_time=False):
    """Compile a relay function into a native library function.

    Parameters
    ----------
    func: Expr
        The function.

    mod: Module
        The Module.

    ctx: Context
        The Context.

    tgt: Target
        The target

    name: String
        The name of the target binary library.

    record_time: Bool
        Time cost to call f?

    Returns
    -------
    result: Function
        A function that, when pass in some values,
        will convert them to the right format and call the compiled func.
    """
    global _LIB
    if isinstance(func, GlobalVar):
        func = mod[func]
    assert isinstance(func, Function)
    compiler = AoTCompiler(mod, tgt)
    func = compiler.optimize(func)
    func = compiler.visit(func)
    lib_name, packed_name = lib_and_func_name(name)
    constants, source_code = to_futil.to_source(mod, func, compiler.gv_map, ctx, packed_name)
    return source_code
