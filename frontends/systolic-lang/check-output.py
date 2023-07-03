import numpy as np
import argparse
import json

parser = argparse.ArgumentParser(description="Process some integers.")
parser.add_argument("file", nargs="?", type=str)
parser.add_argument("-tl", "--top-length", type=int)
parser.add_argument("-td", "--top-depth", type=int)
parser.add_argument("-ll", "--left-length", type=int)
parser.add_argument("-ld", "--left-depth", type=int)
parser.add_argument("-j", "--json-file", type=str)

args = parser.parse_args()

tl = args.top_length
td = args.top_depth
ll = args.left_length
ld = args.left_depth
json_file = args.json_file

assert td == ld, f"Cannot multiply matrices: " f"{tl}x{td} and {ld}x{ll}"

left = np.zeros((ll, ld), dtype="i")
top = np.zeros((td, tl), dtype="i")
json_data = json.load(open(json_file))["memories"]

for r in range(ll):
    for c in range(ld):
        left[r][c] = json_data[f"l{r}"][c]

for r in range(td):
    for c in range(tl):
        top[r][c] = json_data[f"t{c}"][r]

matmul_result = np.matmul(left, top).flatten()

json_result = np.array(json_data["out_mem"])

if np.array_equal(json_result, matmul_result):
    print("Correct")
else:
    print("Incorrect\n. Should have been:\n")
    print(matmul_result)
    print("\nBut got:\n")
    print(json_result)
