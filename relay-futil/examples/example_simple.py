import tvm
from tvm import ir, relay
from aot import compile


def identity():
    """The float32 identity function in Relay.
    """
    x = relay.var('x', shape=())
    f = relay.Function([x], x)
    return f


def simple_example(func):
    # Dump the Relay representation.
    print(func)

    # Compile the function.
    cfunc = compile(func)
    print(cfunc)


if __name__ == '__main__':
    simple_example(identity())
