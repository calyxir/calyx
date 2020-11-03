import tvm
from tvm import relay
from tvm.relay import parser
from compiler import *
import sys


def tensor_0d_add():
    """Add together two variables in Relay.
    """
    x = relay.var('x', shape=(), dtype="int32")
    y = relay.var('y', shape=(), dtype="int32")
    return relay.Function([x, y], relay.add(x, y))


def tensor_1d_add():
    """Add together two 1-dimensional tensors in Relay.
    """
    x = relay.var("x", relay.TensorType((1, 4), "int32"))
    y = relay.var("y", relay.TensorType((1, 4), "int32"))
    return relay.Function([x, y], relay.add(x, y))


def tensor_2d_add():
    """Add together two 2-dimensional tensors in Relay.
    """
    x = relay.var("x", relay.TensorType((2, 4), "int32"))
    y = relay.var("y", relay.TensorType((2, 4), "int32"))
    return relay.Function([x, y], relay.add(x, y))


def assign():
    """Assign a const to a varible
    """
    x = relay.var('x', shape=())
    v1 = relay.log(x)
    v2 = relay.add(v1, x)
    return relay.Function([x], v2)

def mlp_net():
    """The MLP test from Relay.
    """
    from tvm.relay.testing import mlp
    return mlp.get_net(1)


ALL_FUNCS = [tensor_0d_add, tensor_1d_add, tensor_2d_add, mlp_net]


def simple_example():
    func = tensor_0d_add()  # Default if none provided.
    # See if the command line contains a function name.
    for option in ALL_FUNCS:
        if option.__name__ in sys.argv[1:]:
            func = option()
            break

    # Try optimizing the Relay IR with a few built-in passes.
    seq = tvm.transform.Sequential([
        relay.transform.SimplifyExpr(),
        relay.transform.SimplifyInference(),
        relay.transform.ToANormalForm(),
    ])

    mod = tvm.IRModule.from_expr(func)
    mod_opt = seq(mod)
    func = mod_opt['main']

    if '-r' in sys.argv[1:]:
        # Dump the Relay representation (for educational purposes).
        print(func)
    else:
        # Compile the function and print the FuTIL.
        print(compile(func))


if __name__ == '__main__':
    simple_example()
