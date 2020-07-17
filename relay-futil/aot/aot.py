from tvm.relay.function import Function
from . import to_futil


def compile(func, mod, ctx, tgt, name='default'):
    assert isinstance(func, Function)
    source_code = to_futil.to_source(mod, func, {}, ctx, name)
    return source_code
