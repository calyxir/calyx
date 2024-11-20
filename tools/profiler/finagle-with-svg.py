import os
import sys

# Takes in a flame graph svg that is scaled by 1000 and prints a version with fixed cycles.

def main(svg_in):
    oin = open(svg_in, "r")

    for line in oin:
        if line.startswith("<title>"):
            line_split = line.strip().split(" ")
            target_idx = 0
            for i in range(len(line_split)):
                if line_split[i] == "cycles,":
                    target_idx = i-1
            new_number = int(line_split[target_idx].split("(")[1].replace(",", "")) / 1000
            print(" ".join(line_split[0:target_idx]) + " (" + str(new_number) + " " + " ".join(line_split[target_idx+1:]))
        else:
            print(line.strip())


if __name__ == "__main__":
    if len(sys.argv) > 1:
        svg_filename = sys.argv[1]
        main(svg_filename)
    else:
        args_desc = [
            "INPUT_SVG"
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        print("CELLS_JSON: Run the `component_cells` tool")
        sys.exit(-1)