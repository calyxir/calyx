import numpy as np
import argparse
import json


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
    parser.add_argument("file", nargs="?", type=str)
    parser.add_argument("-tl", "--top-length", type=int)
    parser.add_argument("-td", "--top-depth", type=int)
    parser.add_argument("-ll", "--left-length", type=int)
    parser.add_argument("-ld", "--left-depth", type=int)
    parser.add_argument("-p", "--post-op", type=str, default=None)
    parser.add_argument("-j", "--json-file", type=str)

    args = parser.parse_args()

    tl = args.top_length
    td = args.top_depth
    ll = args.left_length
    ld = args.left_depth
    post_op = args.post_op
    json_file = args.json_file

    assert td == ld, f"Cannot multiply matrices: " f"{tl}x{td} and {ld}x{ll}"

    left = np.zeros((ll, ld), "f")
    top = np.zeros((td, tl), "f")
    json_data = json.load(open(json_file))["memories"]

    for r in range(ll):
        for c in range(ld):
            left[r][c] = json_data[f"l{r}"][c]

    for r in range(td):
        for c in range(tl):
            top[r][c] = json_data[f"t{c}"][r]

    matmul_result = np.matmul(left, top)
    if post_op == "leaky-relu":
        matmul_result = np.where(matmul_result > 0, matmul_result, matmul_result * 0.01)
    elif post_op == "relu":
        matmul_result = np.where(matmul_result > 0, matmul_result, 0)

    res = []
    for r in range(ll):
        res.append(list(map(float, json_data[f"out_mem_{r}"])))

    json_result = np.array(res)

    if np.isclose(matmul_result, json_result, atol=1e-3).all():
        print("Correct")
    else:
        print("Incorrect\n. Should have been:\n")
        print(matmul_result)
        print("\nBut got:\n")
        print(json_result)
