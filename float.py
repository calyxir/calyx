# From: https://stackoverflow.com/questions/33483846/how-to-convert-32-bit-binary-to-float

from codecs import decode
import struct
import json
import argparse
import struct


def float_to_bin(num):
    return "".join("{:0>8b}".format(c) for c in struct.pack("!f", num))


def float_to_int(num):
    bin_str = float_to_bin(num)
    return int(bin_str, 2)


def bin_to_float(bin_str):
    f = int(bin_str, 2)
    return struct.unpack("f", struct.pack("I", f))[0]


def int_to_float(num):
    bin_str = bin(num)[2:]
    return bin_to_float(bin_str)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Do sum stuff.")
    parser.add_argument("-j", "--json-file", type=str)
    parser.add_argument("-t", "--target", type=str)
    parser.add_argument("-d", "--debug", type=str, default=None)

    args = parser.parse_args()

    json_file = args.json_file
    target = args.target
    debug = args.debug

    assert target == "int" or target == "float", "expect target int or float"

    if args.debug is not None:
        if target == "float":
            print(int_to_float(int(args.debug)))
        else:
            print(float_to_int(float(args.debug)))
        exit()

    json_data = json.load(open(json_file))
    res = json_data

    if target == "float":
        for entry in res["memories"]:
            res["memories"][entry] = [int_to_float(x) for x in res["memories"][entry]]
    else:
        for entry in res:
            res[entry]["data"] = [float_to_int(x) for x in res[entry]["data"]]
            res[entry]["format"] = {
                "numeric_type": "bitnum",
                "is_signed": False,
                "width": 32,
            }
    json_object = json.dumps(res, indent=2)
    print(json_object)
