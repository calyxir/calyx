import argparse
import json
from calyx import numeric_types


if __name__ == "__main__":
    """
    This is a script to help you know whether the Calyx's systolic array
    generator is giving you the correct answers.

    How to use this script: run Calyx's systolic array generator and get an
    output json. Then run this script on the output json, and this script
    will check the answers against numpy's matrix multiplication implementation.

    Command line arguments are (no json support yet):
    -tl -td -ll -ld are the same as the systolic array arguments.
    -j which is the path to the json you want to check
    """
    parser = argparse.ArgumentParser(description="Process some integers.")
    parser.add_argument("-iw", "--int-width", type=int)
    parser.add_argument("-fw", "--frac-width", type=int)
    parser.add_argument("-bw", "--bit-width", type=int)
    parser.add_argument("-s", "--signed", action="store_true")
    parser.add_argument("-j", "--json_file", type=str)

    args = parser.parse_args()

    int_width = args.int_width
    frac_width = args.frac_width
    bit_width = args.bit_width
    signed = args.signed
    json_file = args.json_file

    assert (
        bit_width == frac_width + int_width
    ), f"Bitwidth {bit_width} should equal: frac_width {frac_width} \
    + int_width {int_width}"

    json_data = json.load(open(json_file))

    for key, value in json_data.items():
        if key != "cycles":
            new_values = [
                numeric_types.bitnum_to_fixed(
                    numeric_types.Bitnum(str(x), is_signed=signed, width=bit_width),
                    int_width=int_width,
                ).string_repr
                for x in value
            ]
            print(new_values)
