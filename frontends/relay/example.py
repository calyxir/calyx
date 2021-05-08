import tvm
from tvm import relay
import relay_visitor
import relay_utils


def tensor_add():
    x = relay.var("x", relay.TensorType((2, 4), "int32"))
    y = relay.var("y", relay.TensorType((2, 4), "int32"))
    return relay.Function([x, y], relay.subtract(x, y))


def expand_dims():
    x = relay.var("x", shape=[512], dtype="int32")
    return relay.Function([x], relay.expand_dims(x, axis=1, num_newaxis=2))


def batch_flatten():
    x = relay.var("x", relay.TensorType((2, 5, 5), "int32"))
    return relay.Function([x], relay.nn.batch_flatten(x))


def batch_matmul():
    x = relay.var("x", shape=[1, 3, 3], dtype="float32")
    y = relay.var("y", shape=[1, 3, 3], dtype="float32")
    return relay.Function([x, y], relay.nn.batch_matmul(x, y))


def bias_add():
    x = relay.var("x", shape=[2, 4], dtype="float32")
    bias = relay.var("bias", shape=[4], dtype="float32")
    return relay.Function([x, bias], relay.nn.bias_add(data=x, bias=bias))


def relu():
    x = relay.var("x", shape=[2, 4], dtype="int32")
    return relay.Function([x], relay.nn.relu(x))


def dense():
    x = relay.var("x", shape=[1, 4096], dtype="int32")
    y = relay.var("y", shape=[10, 4096], dtype="int32")
    return relay.Function([x, y], relay.nn.dense(x, y, units=10))


def softmax():
    x = relay.var("x", shape=[1, 10], dtype="float32")
    return relay.Function([x], relay.nn.softmax(x))


def max_pool2d():
    data = relay.var("data", shape=[2, 2, 4, 4], dtype="int32")
    return relay.Function(
        [data],
        relay.nn.max_pool2d(
            data, padding=[0, 0, 0, 0], strides=[2, 2], pool_size=[2, 2]
        ),
    )


def conv2d():
    d = relay.var("data", shape=[5, 512, 14, 14], dtype="int32")
    w = relay.var("weight", shape=[512, 512, 3, 3], dtype="int32")
    return relay.Function(
        [d, w],
        relay.nn.conv2d(d, w, padding=[1, 1, 1, 1], channels=512, kernel_size=[3, 3]),
    )


def mlp_net():
    """The MLP test from Relay."""
    from tvm.relay.testing import mlp

    return mlp.get_net(1)


def vgg_net():
    """The VGG test from Relay."""
    from tvm.relay.testing import vgg

    return vgg.get_net(
        batch_size=5,
        image_shape=(3, 224, 224),
        num_classes=10,
        dtype="float32",
        num_layers=13,
        batch_norm=True,
    )


FUNCTIONS = {
    "tensor_add": tensor_add,
    "expand_dims": expand_dims,
    "batch_flatten": batch_flatten,
    "batch_matmul": batch_matmul,
    "bias_add": bias_add,
    "relu": relu,
    "dense": dense,
    "softmax": softmax,
    "conv2d": conv2d,
    "max_pool2d": max_pool2d,
    "mlp_net": mlp_net,
    "vgg_net": vgg_net,
}


def pretty_print_functions():
    """Pretty prints the available functions."""
    half = len(FUNCTIONS) // 2
    keys = list(FUNCTIONS.keys())
    for (f1, f2) in zip(keys[:half], keys[half:]):
        whitespace = (16 - len(f1)) * " "
        print(f"- {f1}{whitespace} - {f2}")


def run_example():
    import sys

    """Runs the example.
    Displays Relay IR if `-r` is found.
    Displays Calyx otherwise."""
    input = sys.argv[1:]
    if "-h" in input or not input:
        print(
            f"""
help  -h    Displays available functions to play with.
relay -r    Displays the Relay IR. Displays Calyx otherwise.

Available functions:"""
        )
        pretty_print_functions()
        return

    # See if the command line contains a correct function name.
    func_name = input[0]
    func = FUNCTIONS[func_name]() if func_name in FUNCTIONS.keys() else None
    if func is None:
        print(f"Function `{func_name}` is not a supported.")
        pretty_print_functions()
        return

    if "-r" in input:
        # Dump the Relay IR.
        print(relay_utils.python2relay(func))
    else:
        mod = tvm.IRModule.from_expr(func)
        relay_ir = mod["main"]
        # Compile and dump the Calyx.
        print(relay_visitor.emit_calyx(relay_ir))


if __name__ == "__main__":
    run_example()
