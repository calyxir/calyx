#!/usr/bin/env python3
import pynq
import numpy as np
from fud.stages.json_to_dat import parse_fp_widths


def run(data: Mapping[str, Any]) -> None:
    """Takes in the output of simplejson.loads() and runs pynq using the data provided

    Assumes that data is a properly formatted calyx data file.
    Data file order must match the expected call signature in terms of order
    Also assume that the Any type is a valid json-type equivalent
    """

    # TODO: find xclbin file name/path
    xclbin = None
    ol = pynq.Overlay(xclbin)

    buffers = []
    for mem in data.keys():
        ndarray = np.array(data[mem]["data"], dtype=_data_type(mem, data))
        buffers.append(pynq.allocate(ndarray.shape, dtype=dtype))

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
        if data[mem]["numeric_type"] == "fixed_point":
            width, int_width = parse_fp_widths(data[mem]["format"])
            frac_width = width - int_width
            convert_to_fp = lambda e: e / (2**frac_width)  # noqa : E731
            convert_to_fp(buffers[i])
        # TODO: what to do with arrays? convert back to json?
        # clean up
        del mem
    ol.free()


def _dtype(mem: str, data: Mapping[str:, Any]) -> numpy.dtype:
    # See https://numpy.org/doc/stable/reference/arrays.dtypes.html for typing
    # details
    type_string = "i" if data[mem]["is_signed"] else "u"
    # XXX(nathanielnrn): numpy does not have an unsigned floating point
    # also, you cannot set arbitrary width floating points like you can in
    # calyx programs
    # assumes width is exactly divisible by 8
    byte_size = int(data[mem]["format"]["width"] / 8)
    type_string = type_string + str(byte_size)
    return numpy.dtype(type_string)
