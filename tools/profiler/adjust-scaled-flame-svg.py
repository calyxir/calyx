import sys


# Takes in a flame graph svg that is scaled by 1000 and prints a version with fixed cycles.
# Also adjust the colors of elements based on cells/groups/primitives/control groups
cell_fill = 'fill="rgb(255,0,0)"'
control_group_fill = 'fill="rgb(255,128,0)"'
group_fill = 'fill="rgb(255,255,102)"'
primitive_fill = 'fill="rgb(204,255,153)"'

noColor_opt = "--noColor"
noScale_opt = "--noScale"


def main(svg_in, opts):
    oin = open(svg_in, "r")

    for line in oin:
        function_line = '"Function: " +'
        if function_line in line:
            print(line.replace(function_line, ""))
        elif line.startswith("<title>"):
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
            original_fill = ""
            for i in range(len(line_split)):
                if line_split[i] == "cycles,":
                    target_idx = i - 1
                if line_split[i].startswith("fill="):
                    fill_target_idx = i
                    original_fill = line_split[i]
            if "--noColor" in opts:
                fill = original_fill
            if "--noScale" in opts:
                new_number_str = line_split[target_idx].split("(")[1]  # unmodified
            else:
                new_number = (
                    int(line_split[target_idx].split("(")[1].replace(",", "")) / 1000
                )
                new_number_str = "{:,}".format(new_number)
            print(
                " ".join(line_split[0:target_idx])
                + " ("
                + new_number_str
                + " "
                + " ".join(line_split[target_idx + 1 : fill_target_idx])
                + " "
                + fill
                + " "
                + " ".join(line_split[fill_target_idx + 1 :])
            )
        else:
            print(line.strip())


if __name__ == "__main__":
    if len(sys.argv) > 2:
        svg_filename = sys.argv[1]
        options = sys.argv[2].split(";")
        main(svg_filename, options)
    else:
        args_desc = ["INPUT_SVG", "OPT_LIST"]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        print(
            "OPT_LIST is a ; separated list of options, listed below:"
            '\t- To NOT scale the svg, pass in "--noScale"'
            '\t- To NOT use custom colors, pass in "--noColor"'
        )
        print("")
        sys.exit(-1)
