#!/usr/bin/env python3
"""A standalone tool for executing compiled Xilinx XRT bitstreams.

This tool can be invoked as a subprocess to run a compiled `.xclbin`, which may
be compiled either for RTL simulation or for actual on-FPGA execution. It
consumes and produces fud-style JSON input/output data files but is otherwise
isolated from the rest of fud and can be invoked as a standalone program. This
separate-process model is important so the user (or parent process) can set the
*required* environment variables that the Xilinx toolchain needs to control its
execution mode and to find its support files.

This tool currently uses the `PYNQ`_ Python library, which is meant for
high-level application interaction but is also a fairly stable wrapper around
the underlying XRT libraries. In the future, we can consider replcaing PYNQ
with directly using the `pyxrt`_ library, or abandoning Python altogether and
using the native XRT library directly for simplicity.

A bunch of environment variables have to be set to use xclrun. A minimal
invocation of xclrun looks something like this::

    $ source /scratch/opt/Xilinx/Vitis/2020.2/settings64.sh
    $ source /scratch/opt/xilinx/xrt/setup.sh
    $ export EMCONFIG_PATH=`pwd`
    $ XCL_EMULATION_MODE=hw_emu
    $ XRT_INI_PATH=`pwd`/xrt.ini
    $ python -m fud.xclrun something.xclbin data.json

.. _PYNQ: https://github.com/xilinx/pynq
.. _pyxrt: https://github.com/Xilinx/XRT/blob/master/src/python/pybind11/src/pyxrt.cpp
"""
import argparse
import pynq
import numpy as np
import simplejson as sjson
import sys
from typing import Mapping, Any, Dict
from pathlib import Path
from fud.stages.verilator.json_to_dat import parse_fp_widths, float_to_fixed
from calyx.numeric_types import InvalidNumericType


def mem_to_buf(mem):
    """Convert a fud-style JSON memory object to a PYNQ buffer."""
    ndarray = np.array(mem["data"], dtype=_dtype(mem["format"]))
    buffer = pynq.allocate(ndarray.shape, dtype=ndarray.dtype)
    buffer[:] = ndarray[:]
    return buffer


def buf_to_mem(fmt, buf):
    """Convert a PYNQ buffer to a fud-style JSON memory value."""
    # converts int representation into fixed point
    if fmt["numeric_type"] == "fixed_point":
        width, int_width = parse_fp_widths(fmt)
        frac_width = width - int_width

        def convert_to_fp(value: float):
            float_to_fixed(float(value), frac_width)

        convert_to_fp(buf)
        return list(buf)
    elif fmt["numeric_type"] == "bitnum":
        return list([int(e) for e in buf])

    else:
        raise InvalidNumericType('Fud only supports "fixed_point" and "bitnum".')


def run(xclbin: Path, data: Mapping[str, Any]) -> Dict[str, Any]:
    """Takes in a json data output and runs pynq using the data provided
    returns a dictionary that can be converted into json

    `xclbin` is path to relevant xclbin file.
    Assumes that data is a properly formatted calyx data file.
    Data file order must match the expected call signature in terms of order
    Also assume that the data Mapping values type are valid json-type equivalents
    """

    # Load the PYNQ overlay from the .xclbin file, raising a FileNotFoundError
    # if the file does not exist.
    ol = pynq.Overlay(str(xclbin.resolve(strict=True)))

    # Send all the input data.
    buffers = [mem_to_buf(mem) for mem in data.values()]
    for buffer in buffers:
        buffer.sync_to_device()

    # Run the kernel.
    kernel = getattr(ol, list(ol.ip_dict)[0])  # Like ol.Toplevel_1
    # XXX(nathanielnrn) 2022-07-19: timeout is not currently used anywhere in
    # generated verilog code, passed in because kernel.xml is generated to
    # expect it as an argument
    timeout = 1000
    kernel.call(timeout, *buffers)

    # Collect the output data.
    for buf in buffers:
        buf.sync_from_device()
    mems = {
        name: buf_to_mem(data[name]["format"], buf) for name, buf in zip(data, buffers)
    }

    # PYNQ recommends explicitly freeing its resources.
    del buffers
    ol.free()

    return {"memories": mems}


def _dtype(fmt) -> np.dtype:
    # See https://numpy.org/doc/stable/reference/arrays.dtypes.html for typing
    # details
    type_string = "i" if fmt["is_signed"] else "u"
    byte_size = int(fmt["width"] / 8)
    type_string = type_string + str(byte_size)
    return np.dtype(type_string)


def xclrun():
    # Parse command-line arguments.
    parser = argparse.ArgumentParser(
        description="run a compiled XRT program",
    )
    parser.add_argument("bin", metavar="XCLBIN", help="the .xclbin binary file to run")
    parser.add_argument("data", metavar="DATA", help="the JSON input data file")
    parser.add_argument(
        "--out",
        "-o",
        metavar="FILE",
        help="write JSON results to a file instead of stdout",
    )
    args = parser.parse_args()

    # Load the input JSON data file.
    with open(args.data) as f:
        in_data = sjson.load(f, use_decimal=True)

    # Run the program.
    out_data = run(Path(args.bin), in_data)

    # Dump the output JSON data.
    outfile = open(args.out, "w") if args.out else sys.stdout
    sjson.dump(out_data, outfile, indent=2, use_decimal=True)


if __name__ == "__main__":
    xclrun()
