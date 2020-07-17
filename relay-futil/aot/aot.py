from tvm.relay.function import Function
from . import to_futil


def compile(func):
    assert isinstance(func, Function)
    source_code = to_futil.to_source(func)
    return source_code
