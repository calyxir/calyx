#!/usr/bin/env python3
import argparse
import json
import numpy as np
import shlex
import subprocess
import os
from typing import Mapping, Any, Dict, List
from pathlib import Path
from fud.stages.verilator.json_to_dat import parse_fp_widths, float_to_fixed
from fud.errors import InvalidNumericType


# XXX(nathanielnrn): Should xclbin typing only be pathlib.Path, or also accept strings?
def run(xclbin_path: Path, data: Mapping[str, Any]) -> Dict[str, Any]:
    """Takes in a json data output and runs pynq using the data provided
    returns a dictionary that can be converted into json

    `xclbin` is path to relevant xclbin file.
    Assumes that data is a properly formatted calyx data file.
    Data file order must match the expected call signature in terms of order
    Also assume that the data Mapping values type are valid json-type equivalents
    """
    import pynq

    # pynq.Overlay expects a str
    # Raises FileNotFoundError if xclbin file does not exist
    ol = pynq.Overlay(str(xclbin_path.resolve(strict=True)))

    buffers = []
    for mem in data.keys():
        ndarray = np.array(data[mem]["data"], dtype=_dtype(mem, data))
        shape = ndarray.shape
        buffer = pynq.allocate(shape, dtype=ndarray.dtype)
        buffer[:] = ndarray[:]
        buffers.append(buffer)

    for buffer in buffers:
        buffer.sync_to_device()

    # Equivalent to setting kernel = ol.<Presumably 'Toplevel_1'>
    kernel = getattr(ol, list(ol.ip_dict)[0])
    # XXX(nathanielnrn) 2022-07-19: timeout is not currently used anywhere in
    # generated verilog code, passed in because kernel.xml is generated to
    # expect it as an argument
    timeout = 1000
    kernel.call(timeout, *buffers)

    output = {"memories": {}}
    # converts needed data from buffers and adds to json output
    for i, mem in enumerate(data.keys()):
        buffers[i].sync_from_device()
        # converts int representation into fixed point
        if data[mem]["format"]["numeric_type"] == "fixed_point":
            width, int_width = parse_fp_widths(data[mem]["format"])
            frac_width = width - int_width

            def convert_to_fp(value: float):
                float_to_fixed(float(value), frac_width)

            convert_to_fp(buffers[i])
            output["memories"][mem] = list((buffers[i]))
        elif data[mem]["format"]["numeric_type"] == "bitnum":
            output["memories"][mem] = list(map(lambda e: int(e), buffers[i]))

        else:
            raise InvalidNumericType('Fud only supports "fixed_point" and "bitnum".')

    # PYNQ recommends deleting buffers and freeing overlay
    del buffers
    ol.free()
    return output


def _dtype(mem: str, data: Mapping[str, Any]) -> np.dtype:
    # See https://numpy.org/doc/stable/reference/arrays.dtypes.html for typing
    # details
    type_string = "i" if data[mem]["format"]["is_signed"] else "u"
    byte_size = int(data[mem]["format"]["width"] / 8)
    type_string = type_string + str(byte_size)
    return np.dtype(type_string)


def _get_env(script: str, vars: List[str]) -> Dict[str, str]:
    """Run a bash script and collect resulting environment variables.

    `script` should be a path to a bash script we'll `source` to set some
    environment variables. Then, we collect all the named `vars` and return
    their values in a dictionary.
    """
    cmd_parts = [f'source {shlex.quote(script)}'] + \
        [f" ; printf '%s\\0' \"${v}\"" for v in vars]
    cmd = ''.join(cmd_parts)
    proc = subprocess.run(['bash', '-c', cmd], capture_output=True, check=True)
    values = proc.stdout.split(b'\0')[:-1]
    return {k: v.decode() for k, v in zip(vars, values)}


def vitis_env(path: str):
    """Load the environment for Xilinx Vitis."""
    setup_script = os.path.join(path, 'settings64.sh')
    return _get_env(setup_script, [
        'XILINX_VIVADO',
        'XILINX_HLS',
        'XILINX_VITIS',
    ])


def xrt_env(path: str):
    """Load the environment for XRT."""
    setup_script = os.path.join(path, 'setup.sh')
    return _get_env(setup_script, [
        'XILINX_XRT',
    ])


def pynq_exec():
    """Command-line entry point for running xclbin files.
    """
    parser = argparse.ArgumentParser(description='Run an xclbin program.')
    parser.add_argument('xclbin', type=str, metavar='XCLBIN',
                        help='Xilinx compiled binary file')
    parser.add_argument('data', type=str, metavar='JSON',
                        help='JSON input data file')
    parser.add_argument('--vitis', type=str, metavar='DIR',
                        help='Xilinx Vitis installation directory')
    parser.add_argument('--xrt', type=str, metavar='DIR',
                        help='XRT installation directory')
    args = parser.parse_args()

    if args.vitis:
        os.environ.update(vitis_env(args.vitis))
    if args.xrt:
        os.environ.update(xrt_env(args.xrt))

    # Parse the data and run.
    with open(args.data) as f:
        data = json.load(f)
    run(Path(args.xclbin), data)


if __name__ == '__main__':
    pynq_exec()
