import onnx
import relay_visitor
import tvm
import tvm.relay as relay
import numpy as np
import simplejson as sjson
from image_processing import preprocess_image
from calyx.py_ast import Import

WIDTH = 32
FRAC_WIDTH = 16
IS_SIGNED = True


def write_data(relay_ir, input, input_name: str, params, filename: str):
    """Writes the `.data` file to `filename` for Calyx simulation, with
    the corresponding parameter values. `input` is the data being
    classified, and `input_name` is its name. `params` are the
    parameters from the ONNX model."""

    input_name = relay_visitor.rename_relay_var(input_name)

    # Get the memories from the Calyx program.
    data = relay_visitor.get_program_dat_memories(relay_ir)

    # Write the input.
    data[input_name] = {
        "data": input.tolist(),
        "format": {
            "numeric_type": "fixed_point",
            "is_signed": IS_SIGNED,
            "width": WIDTH,
            "frac_width": FRAC_WIDTH,
        },
    }

    # Write the actual parameter values.
    for name, value in params.items():
        # The exact same operations are done on names of variables in relay_visitor.py
        new_name = relay_visitor.rename_relay_var(name)
        data[new_name] = {
            "data": value.asnumpy().tolist(),
            "format": {
                "numeric_type": "fixed_point",
                "is_signed": IS_SIGNED,
                "width": WIDTH,
                "frac_width": FRAC_WIDTH,
            },
        }

    with open(filename, "w") as file:
        sjson.dump(data, file, sort_keys=True, indent=2)


def write_calyx(relay_ir, filename: str, save_mem=True):
    """Writes the Calyx program lowered
    from `relay_ir` to `filename`."""
    (dahlia_defs, prog) = relay_visitor.emit_calyx(relay_ir, save_mem)
    with open(filename, "w") as file:
        imports = [
            Import("primitives/core.futil"),
            Import("primitives/binary_operators.futil"),
            Import("primitives/math.futil"),
            Import("primitives/memories/seq.futil"),
        ]
        for imp in imports:
            file.writelines(imp.doc())
        file.writelines(dahlia_defs)
        file.writelines(prog.doc())


def write_relay(relay_ir, filename: str):
    """Writes the `relay_ir` to `filename`."""
    with open(filename, "w") as file:
        file.writelines(str(relay_ir))


def run_net(net_name: str, input, onnx_model_path: str, output: str, save_mem=True):
    """Runs the net with name `net_name` to classify the `input`
    with the ONNX model at `onnx_model_path`.
    - If `output` is "calyx":
      (1) Writes the Calyx program to <net_name>.futil
      (2) Writes the data for Calyx simulation to <net_name>.data
    - If output is "tvm", executes the Relay program with the TVM executor.
    - If output is "relay", writes the Relay IR to <net_name>.relay
    """
    onnx_model = onnx.load(onnx_model_path)
    input_name = onnx_model.graph.input[0].name

    shape_dict = {input_name: data.shape}
    (mod, params) = relay.frontend.from_onnx(onnx_model, shape_dict)

    # Assumes the Relay IR is not already in A-normal Form.
    # SimplifyInference() gets rid of dropout() calls
    transforms = tvm.transform.Sequential(
        [relay.transform.SimplifyInference(), relay.transform.ToANormalForm()]
    )
    mod = transforms(mod)

    output = {"tvm", "calyx", "relay"} if output == "all" else {output}
    if "calyx" in output:
        # Save the parameters of the model to
        # a file for Calyx simulation.
        data_name = f"{net_name}.data"
        calyx_name = f"{net_name}.futil"

        print(f"...writing the Calyx data to: {data_name}")
        write_data(mod, data, input_name, params, data_name)

        print(f"...writing the Calyx program to: {calyx_name}")
        write_calyx(mod, calyx_name, save_mem)
    if "relay" in output:
        relay_name = f"{net_name}.relay"
        print(f"...writing the Relay IR to: {relay_name}")
        write_relay(mod, relay_name)
    if "tvm" in output:
        with tvm.transform.PassContext(opt_level=1):
            intrp = relay.build_module.create_executor("graph", mod, tvm.cpu(0))

        # Execute the ONNX model with the given parameters.
        assert isinstance(
            data, np.ndarray
        ), f"The input type, {type(data)}, should be `class '<numpy.ndarray>'`."
        tvm_output = intrp.evaluate()(tvm.nd.array(data.astype("float32")), **params)

        np.set_printoptions(suppress=True, precision=16)
        print(f"TVM classification output for {net_name}:\n{tvm_output}")


if __name__ == "__main__":
    # Script for running an ONNX model.
    import argparse

    parser = argparse.ArgumentParser(description="ONNX to Calyx")
    parser.add_argument("-n", "--net_name", required=True, help="Name of your net.")
    parser.add_argument(
        "-d",
        "--dataset",
        required=True,
        help='Dataset used, e.g. "mnist". Needed for image preprocessing.',
    )
    parser.add_argument("-i", "--image", required=True, help="Path to the input image.")
    parser.add_argument(
        "-onnx", "--onnx_model", required=True, help="Path to the ONNX model."
    )
    parser.add_argument(
        "-o",
        "--output",
        required=True,
        choices={"calyx", "tvm", "relay", "all"},
        help="Choices: `calyx`, `tvm`, `relay`, or `all` the above.",
    )
    parser.add_argument(
        "-s",
        "--save-mem",
        required=False,
        help="boolean arguement to determine whether to save the memory you use.  \
        Default value is set to True ",
    )

    args = vars(parser.parse_args())

    # The name of your net.
    net_name = args["net_name"]
    # The filepath to your input data.
    input_path = args["image"]

    # The dataset for which the classification is occurring, e.g. "mnist".
    dataset = args["dataset"]
    # Preprocess the data for classification.
    data = preprocess_image(input_path, dataset)

    # The filepath to the ONNX model.
    onnx_model_path = args["onnx_model"]

    # Determines which output you want.
    output = args["output"]

    # Determines whether you want to save memory or not since save_mem is
    # an optional argument, we want default setting of save_mem to be true
    save_mem = (
        args["save_mem"] is None
        or args["save_mem"] == "True"
        or args["save_mem"] == "true"
    )

    # Runs the net and prints the classification output.
    run_net(net_name, data, onnx_model_path, output, save_mem)
