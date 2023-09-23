import numpy as np
import argparse
import json


if __name__ == "__main__":
    """ """
    parser = argparse.ArgumentParser(description="Process some integers.")
    parser.add_argument("-n", "--dim", type=int)
    parser.add_argument("-s", "--start", type=int)

    args = parser.parse_args()

    n = args.dim
    cur_index = args.start

    inputs = "\n  @clk clock: 1,\n  @reset reset: 1,\n"
    outputs = ""

    for _ in range(2 * n):
        outputs += f"  var{cur_index}_ready: 1,\n"
        inputs += f"  var{cur_index}_valid: 1,\n  var{cur_index}_bits: 32,\n"
        cur_index += 1

    for i in range(n):
        suffix = "," if i != (n - 1) else ""
        inputs += f"  var{cur_index}_ready: 1{suffix}\n"
        outputs += f"  var{cur_index}_valid: 1,\n  var{cur_index}_bits: 32{suffix}\n"
        cur_index += 1
    module_str = f"primitive hec_systolic_array_{n}\n("
    module_str += inputs
    module_str += ") -> (\n"
    module_str += outputs
    module_str += ");"
    print(module_str)
