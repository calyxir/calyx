#!/usr/bin/python3
import sys


# Usage: `./vcdump.py <path-to-file>` will add dump lines required to get icarus to create vcds


def replace_line(file_path, old_line, new_line):
    try:
        with open(file_path, "r") as file:
            lines = file.readlines()

        with open(file_path, "w") as file:
            for line in lines:
                if line.strip() == old_line.strip():
                    file.write(new_line + "\n")
                else:
                    file.write(line)
        print(f"Replacement in '{file_path}' successful.")
    except Exception as e:
        print(f"Error: {e}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python script.py <path_to_file.v>")
    else:
        file_path = sys.argv[1]
        old_line = "// COMPONENT END: main"
        new_line = """
`ifdef COCOTB_SIM
  initial begin
    $dumpfile ("out.vcd");
    $dumpvars (0, wrapper);
    #1;
  end
`endif
// COMPONENT END: main
        """
        replace_line(file_path, old_line, new_line)
