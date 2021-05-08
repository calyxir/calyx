import onnx
import relay_visitor
import tvm
import tvm.relay as relay
import numpy as np
import simplejson as sjson
from image_processing import preprocess_image


def write_data(relay_ir, input, input_name: str, params, filename: str):
    """Writes the `.data` file to `filename` for Calyx simulation, with
    the corresponding parameter values. `input` is the data being
    classified, and `input_name` is its name. `params` are the
    parameters from the ONNX model."""
    # Get the memories from the Calyx program.
    data = relay_visitor.get_program_dat_memories(relay_ir)

    width = 32
    frac_width = 16
    is_signed = True

    # Write the input.
    data[input_name] = {
        "data": input.tolist(),
        "format": {
            "numeric_type": "fixed_point",
            "is_signed": is_signed,
            "width": width,
            "frac_width": frac_width
        }
    }

    # Write the actual parameter values.
    for name, value in params.items():
        data[name] = {
            "data": value.asnumpy().tolist(),
            "format": {
                "numeric_type": "fixed_point",
                "is_signed": is_signed,
                "width": width,
                "frac_width": frac_width
            }
        }

    with open(filename, "w") as file:
        sjson.dump(data, file, sort_keys=True, indent=2)


def write_calyx(relay_ir, filename: str):
    """Writes the Calyx program lowered
    from `relay_ir` to `filename`."""
    calyx_program = relay_visitor.emit_calyx(relay_ir)
    with open(filename, "w") as file:
        file.writelines(calyx_program)


def write_relay(relay_ir, filename: str):
    """Writes the `relay_ir` to `filename`."""
    with open(filename, "w") as file:
        file.writelines(str(relay_ir))


def run_net(net_name: str, input, onnx_model_path: str, write_calyx_data: bool):
    """Runs the net with name `net_name` to classify the `input`
    with the ONNX model at `onnx_model_path`. If `write_calyx_data` is True:
      (1) Writes the Calyx program to <net_name>.futil
      (2) Writes the data for Calyx simulation to <net_name>.data
      (3) Writes the Relay IR to <net_name>.relay
    """
    onnx_model = onnx.load(onnx_model_path)
    input_name = onnx_model.graph.input[0].name

    shape_dict = {input_name: data.shape}
    mod, params = relay.frontend.from_onnx(onnx_model, shape_dict)

    # Assumes the Relay IR is not already in A-normal Form.
    transforms = tvm.transform.Sequential([relay.transform.ToANormalForm()])
    mod = transforms(mod)

    if write_calyx_data:
        # Save the parameters of the model to
        # a file for Calyx simulation.
        write_data(mod, data, input_name, params, f"{net_name}.data")
        # Save the Calyx program to a file.
        write_calyx(mod, f"{net_name}.futil")
        # Save the Relay IR to a file.
        write_relay(mod, f"{net_name}.relay")

    with tvm.transform.PassContext(opt_level=1):
        intrp = relay.build_module.create_executor("graph", mod, tvm.cpu(0))

    # Execute the ONNX model with the given parameters.
    assert isinstance(data, np.ndarray), f"The input type, {type(data)}, should be `class '<numpy.ndarray>'`."
    tvm_output = intrp.evaluate()(tvm.nd.array(data.astype("float32")), **params)

    np.set_printoptions(suppress=True, precision=16)
    print(f"TVM classification output for {net_name}:\n{tvm_output}")


if __name__ == "__main__":
    # Script for running an ONNX model.
    import argparse
    parser = argparse.ArgumentParser(description="ONNX to Calyx")
    parser.add_argument("-n", "--net_name", required=True, help="Name of your net.")
    parser.add_argument("-d", "--dataset", required=True, help="The dataset used. Needed for image preprocessing.")
    parser.add_argument("-i", "--image", required=True, help="Path to the input image.")
    parser.add_argument("-ox", "--onnx_model", required=True, help="Path to the ONNX model.")
    parser.add_argument("-c", "--calyx_write", required=True, help="Writes Calyx program and simulation data.")

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

    # If True, writes the Calyx program and necessary `.dat` file
    # for simulation. Also writes the Relay IR for reference.
    write_calyx_data = args["calyx_write"]

    # Runs the net and prints the classification output.
    run_net(net_name, data, onnx_model_path, write_calyx_data)
