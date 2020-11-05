import tvm
from tvm import relay
from compiler import *
import sys


def add():
    """Add together two variables in Relay.
    """
    x = relay.var('x', shape=(), dtype="int32")
    y = relay.var('y', shape=(), dtype="int32")
    return relay.Function([x, y], relay.add(x, y))


def tensor_add():
    """Add together two 2-dimensional tensors in Relay.
    """
    x = relay.var("x", relay.TensorType((2, 4), "int32"))
    y = relay.var("y", relay.TensorType((2, 4), "int32"))
    return relay.Function([x, y], relay.add(x, y))


def batch_flatten():
    """Flattens all dimensions except for the batch dimension.
    """
    x = relay.var("x", relay.TensorType((2, 5, 5), "int32"))
    return relay.Function([x], relay.nn.batch_flatten(x))


def mlp_net():
    """The MLP test from Relay.
    """
    from tvm.relay.testing import mlp
    return mlp.get_net(1)


ALL_FUNCS = [add, tensor_add, batch_flatten, mlp_net]
FUNC_NAMES = list(map(lambda x: x.__name__, ALL_FUNCS))


def simple_example():
    if '-h' in sys.argv[1:]:
        supported_functions = []
        print("- To see FuTIL output:\n$ python3 example.py <function_name>")
        print("- To see Relay IR:\n$ python3 example.py <function_name> -r")
        print("\n- Supported function names:")
        for f in FUNC_NAMES: print(f'    {f}')
        return
    func = None
    # See if the command line contains a function name.
    for option in ALL_FUNCS:
        if option.__name__ in sys.argv[1:]:
            func = option()
            break
    if func == None:
        print("For help:\n$ python3 example.py -h")
        return

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
