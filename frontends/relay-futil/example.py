import tvm
from tvm import relay
from compiler import *
import sys


def add():
    x = relay.var('x', shape=(), dtype="int32")
    y = relay.var('y', shape=(), dtype="int32")
    return relay.Function([x, y], relay.add(x, y))


def tensor_subtract():
    x = relay.var("x", relay.TensorType((2, 4), "int32"))
    y = relay.var("y", relay.TensorType((2, 4), "int32"))
    return relay.Function([x, y], relay.subtract(x, y))


def expand_dims():
    x = relay.var('x', shape=[512], dtype='int32')
    return relay.Function([x], relay.expand_dims(x, axis=1, num_newaxis=2))


def batch_flatten():
    x = relay.var("x", relay.TensorType((2, 5, 5), "int32"))
    return relay.Function([x], relay.nn.batch_flatten(x))


def batch_matmul():
    x = relay.var('x', shape=[1, 3, 3], dtype='float32')
    y = relay.var('y', shape=[1, 3, 3], dtype='float32')
    return relay.Function([x, y], relay.nn.batch_matmul(x, y))


def bias_add():
    x = relay.var('x', shape=[2, 4], dtype='float32')
    bias = relay.var('bias', shape=[4], dtype='float32')
    return relay.Function([x, bias], relay.nn.bias_add(data=x, bias=bias))


def relu():
    x = relay.var('x', shape=[2, 4], dtype='int32')
    return relay.Function([x], relay.nn.relu(x))


def mlp_net():
    """The MLP test from Relay."""
    from tvm.relay.testing import mlp
    return mlp.get_net(1)


def vgg_net():
    """The VGG test from Relay."""
    from tvm.relay.testing import vgg
    return vgg.get_net(batch_size=1, image_shape=(3, 224, 224), num_classes=10, dtype='int32', num_layers=11,
                       batch_norm=True)


ALL_FUNCS = [add, tensor_subtract, expand_dims, batch_flatten, batch_matmul, bias_add, relu, mlp_net, vgg_net]
FUNC_NAMES = list(map(lambda x: x.__name__, ALL_FUNCS))


def run_example():
    input = sys.argv[1:]
    if '-h' in input or input == []:
        print("- To see FuTIL output:\n$ python3 example.py <function_name>")
        print("- To see Relay IR:\n$ python3 example.py <function_name> -r")
        print("\n- Supported functions:")
        (lambda x: print(', '.join(x)))(FUNC_NAMES)
        return
    func = None
    # See if the command line contains a function name.
    for option in ALL_FUNCS:
        if option.__name__ in input:
            func = option()
            break
    if func == None:
        print(f'Function {input} is not a supported. To see a list of functions:\n$ python3 example.py -h')
        return

    # Try optimizing the Relay IR with a few built-in passes.
    seq = tvm.transform.Sequential([
        relay.transform.SimplifyExpr(),
        relay.transform.SimplifyInference(),
        relay.transform.ToANormalForm(),
    ])

    mod_opt = tvm.IRModule.from_expr(func)
    mod_opt = seq(mod_opt)
    func = mod_opt['main']
    if '-r' in input:
        # Dump the Relay representation (for educational purposes).
        print(func)
    else:
        # Compile the function and print the FuTIL.
        print(compile(func))


if __name__ == '__main__':
    run_example()
