#!/usr/bin/env python3
import pynq
import numpy as np
from typing import Mapping, Any
from fud.stages.json_to_dat import parse_fp_widths


def run(xclbin: data: Mapping[str, Any]) -> None:
    """Takes in the output of simplejson.loads() and runs pynq using the data provided

    Assumes that data is a properly formatted calyx data file.
    Data file order must match the expected call signature in terms of order
    Also assume that the Any type is a valid json-type equivalent
    """

    # TODO: find xclbin file name/path
    xclbin = 
    ol = pynq.Overlay(xclbin)

    buffers = []
    for mem in data.keys():
        print(f"{mem} is " + str(mem))
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

    for buffer in buffers:
        buffer.sync_from_device()
    for i, mem in enumerate(data.keys()):
        # converts int representation into fixed point
        if data[mem]["format"]["numeric_type"] == "fixed_point":
            width, int_width = parse_fp_widths(data[mem]["format"])
            frac_width = width - int_width
            convert_to_fp = lambda e: e / (2**frac_width)  # noqa : E731
            convert_to_fp(buffers[i])
        # TODO: what to do with arrays? convert back to json? for now prints
        # clean up
        del mem
    ol.free()


def _dtype(mem: str, data: Mapping[str, Any]) -> np.dtype:
    # See https://numpy.org/doc/stable/reference/arrays.dtypes.html for typing
    # details
    type_string = "i" if data[mem]["format"]["is_signed"] else "u"
    byte_size = int(data[mem]["format"]["width"] / 8)
    type_string = type_string + str(byte_size)
    return np.dtype(type_string)
