import tvm
from tvm import relay
from tvm.relay import parser
from compiler import *
import sys


def identity():
    """Definition of a relay variable."""
    x = relay.var('x', shape=(), dtype="int32")
    f = relay.Function([x], x)
    return f


def const():
    """A simple constant function in Relay.
    """
    return parser.fromtext('v0.0.4 fn(){%0 = 42; %0}')


# def tensor_1d_add():
#     """Add together two 1-dimensional tensors in Relay.
#     """
#     x = relay.var("x", relay.TensorType((1, 4), "int32"))
#     y = relay.var("y", relay.TensorType((1, 4), "int32"))
#     return relay.Function([x, y], relay.add(x, y))
#
#
# def tensor_2d_add():
#     """Add together two 2-dimensional tensors in Relay.
#     """
#     x = relay.var("x", relay.TensorType((2, 4), "int32"))
#     y = relay.var("y", relay.TensorType((2, 4), "int32"))
#     return relay.Function([x, y], relay.add(x, y))
#
#
# def add():
#     """Add together two constants in Relay.
#     """
#     return relay.Function([], relay.add(relay.const(37), relay.const(5)))
#
#
# def add_var():
#     """Add together two variables
#     """
#     x = relay.var('x', shape=())
#     y = relay.var('y', shape=())
#     return relay.Function([x, y], relay.add(x, y))
#
#
# def assign():
#     """Assign a const to a varible
#     """
#     x = relay.var('x', shape=())
#     v1 = relay.log(x)
#     v2 = relay.add(v1, x)
#     return relay.Function([x], v2)
#
#
# def conv2d(weight=None, **kwargs):
#     name = 'test'
#     if not weight:
#         weight = relay.var(name + "_weight")
#     data = relay.var("data", relay.TensorType((5, 5), "float32"))
#     return relay.Function([data, weight], relay.nn.conv2d(data, weight, **kwargs))
#
#
# def mlp_net():
#     """The MLP test from Relay.
#     """
#     from tvm.relay.testing import mlp
#     return mlp.get_net(1)


ALL_FUNCS = [identity, const]


def simple_example():
    # See if the command line contains a function name.
    for option in ALL_FUNCS:
        if option.__name__ in sys.argv[1:]:
            func = option()
            break
    else:
        variable()

    if '-o' in sys.argv[1:]:
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
