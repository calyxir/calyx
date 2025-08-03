import sys


# Takes in a flame graph svg that is scaled by 1000 and prints a version with fixed cycles.
# Also adjust the colors of elements based on cells/groups/primitives/control groups
cell_fill = 'fill="rgb(255,0,0)"'
control_group_fill = 'fill="rgb(255,128,0)"'
group_fill = 'fill="rgb(255,255,102)"'
primitive_fill = 'fill="rgb(204,255,153)"'

def main(svg_in):
    oin = open(svg_in, "r")

    for line in oin:
        if line.startswith("<title>"):
            if "(primitive)" in line:
                fill = primitive_fill
            elif "[" in line or "<title>main" in line:
                fill = cell_fill
            elif "(ctrl)" in line:
                fill = control_group_fill
            else:
                fill = group_fill
            line_split = line.strip().split(" ")
            target_idx = 0
            fill_target_idx = 0
            for i in range(len(line_split)):
                if line_split[i] == "cycles,":
                    target_idx = i - 1
                if line_split[i].startswith("fill="):
                    fill_target_idx = i
            new_number = (
                int(line_split[target_idx].split("(")[1].replace(",", "")) / 1000
            )
            print(
                " ".join(line_split[0:target_idx])
                + " ("
                + "{:,}".format(new_number)
                + " "
                + " ".join(line_split[target_idx + 1 : fill_target_idx])
                + " " + fill + " "
                + " ".join(line_split[fill_target_idx + 1 :])
            )
        else:
            print(line.strip())


if __name__ == "__main__":
    if len(sys.argv) > 1:
        svg_filename = sys.argv[1]
        main(svg_filename)
    else:
        args_desc = ["INPUT_SVG"]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        print("CELLS_JSON: Run the `component_cells` tool")
        sys.exit(-1)
