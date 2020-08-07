from tvm import relay
from tvm import te
from tvm.relay import parser
import relay2futil
import sys


def identity():
    """The float32 identity function in Relay.
    """
    x = relay.var('x', shape=())
    f = relay.Function([x], x)
    return f


def const():
    """A simple constant function in Relay.
    """
    return parser.fromtext('fn(){42}') 


def add():
    """Add together two constants in Relay.
    """
    return relay.Function([], relay.add(relay.const(37), relay.const(5)))

def add_var():
    """Add together two variables
    """
    x = relay.var('x', shape=())
    y = relay.var('y', shape=())
    return relay.Function([x, y], relay.add(x, y))

def assign():
    """Assign a const to a varible
    """
    x = relay.var('x', shape=())
    v1 = relay.log(x)
    v2 = relay.add (v1,x)
    return relay.Function([x], v2)

def conv2d( weight=None, **kwargs):
    name = 'test'
    if not weight:
        weight = relay.var(name + "_weight")
    data = relay.var("data", relay.TensorType((5, 5), "float32"))
    return relay.Function([data, weight], relay.nn.conv2d(data, weight, **kwargs))


def broadcast_add(a_shape=(4, 1), b_shape=(4, 4)):
    """The `broadcast_add` example from the "D2L book," which adds a
    vector to every column of a matrix.
    """
    # https://tvm.d2l.ai/chapter_common_operators/broadcast_add.html
    assert len(a_shape) == 2 and len(b_shape) == 2, \
        "broadcast tensors should both be 2-dimension"
    for i in range(len(a_shape)):
        assert a_shape[i] == b_shape[i] \
            or a_shape[i] == 1 or b_shape[i] == 1, \
            "tensor shapes do not fit for broadcasting"
    A = te.placeholder(a_shape, name='A')
    B = te.placeholder(b_shape, name='B')
    m = a_shape[0] if b_shape[0] == 1 else b_shape[0]
    n = a_shape[1] if b_shape[1] == 1 else b_shape[1]

    def f(x, y):
        return A[0 if a_shape[0] == 1 else x,
                 0 if a_shape[1] == 1 else y] + \
               B[0 if b_shape[0] == 1 else x,
                 0 if b_shape[1] == 1 else y]

    C = te.compute((m, n), f, name='C')
    return A, B, C


ALL_FUNCS = [identity, const, add, add_var, assign, conv2d]
def simple_example():
    # See if the command line contains a function name.
    for option in ALL_FUNCS:
        if option.__name__ in sys.argv[1:]:
            func = option()
            break
    else:
        func = add()  # The default for no argument.

    if '-r' in sys.argv[1:]:
        # Dump the Relay representation (for educational purposes).
        print(func)
    else:
        # Compile the function and print the FuTIL.
        print(relay2futil.compile(func))


if __name__ == '__main__':
    simple_example()
