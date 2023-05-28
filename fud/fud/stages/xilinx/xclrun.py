#!/usr/bin/env python3
"""A standalone tool for executing compiled Xilinx XRT bitstreams.

This tool can be invoked as a subprocess to run a compiled `.xclbin`, which may
be compiled either for RTL simulation or for actual on-FPGA execution. It
consumes and produces fud-style JSON input/output data files but is otherwise
isolated from the rest of fud and can be invoked as a standalone program. This
separate-process model is important so the user (or parent process) can set the
*required* environment variables that the Xilinx toolchain needs to control its
execution mode and to find its support files.

This tool currently uses the `PYNQ`_ Python library, which is meant for high-level
application interaction but is also a fairly stable wrapper around the
underlying XRT libraries. In the future, we can consider replcaing PYNQ with
directly using the `pyxrt`_ library, or abandoning Python altogether and using
the native XRT library directly for simplicity.

.. _PYNQ: https://github.com/xilinx/pynq
.. _pyxrt: https://github.com/Xilinx/XRT/blob/master/src/python/pybind11/src/pyxrt.cpp
"""
import argparse
import pynq
import numpy as np
import simplejson as sjson
from typing import Mapping, Any, Dict
from pathlib import Path
from fud.stages.verilator.json_to_dat import parse_fp_widths, float_to_fixed
from fud.errors import InvalidNumericType


def run(xclbin_path: Path, data: Mapping[str, Any]) -> Dict[str, Any]:
    """Takes in a json data output and runs pynq using the data provided
    returns a dictionary that can be converted into json

    `xclbin` is path to relevant xclbin file.
    Assumes that data is a properly formatted calyx data file.
    Data file order must match the expected call signature in terms of order
    Also assume that the data Mapping values type are valid json-type equivalents
    """

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


def xclrun():
    # Parse command-line arguments.
    parser = argparse.ArgumentParser(
        description='run a compiled XRT program',
    )
    parser.add_argument('bin', metavar='XCLBIN',
                        help='the .xclbin binary file to run')
    parser.add_argument('data', metavar='DATA',
                        help='the JSON input data file')
    args = parser.parse_args()
    
    # Load the input JSON data file.
    with open(args.data) as f:
        in_data = sjson.load(f, use_decimal=True)
    
    # Run the program.
    out_data = run(Path(args.bin), in_data)
    
    # Dump the output JSON data.
    sjson.dump(out_data, sys.stdout)


if __name__ == "__main__":
    xclrun()
