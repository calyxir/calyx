import json
import os
import subprocess
import sys

# Generates the arguments for m4 based on the JSON data
def generate_m4_arguments(inst):
    args = []
    # get the parameters to the file
    for param in inst["params"]:
        key = param["param_name"]
        value = param["param_value"]
        args.append(f"-D{key}={value}")

    # get the filename
    firrtl_file_path = os.path.join(sys.path[0], "templates", inst['name'] + ".fir")
    args.append(firrtl_file_path)

    return args


def main():
    in_file = open(sys.argv[1])
    primitive_insts = json.load(in_file)
    for inst in primitive_insts:
        m4_args = ["m4"]
        m4_args += generate_m4_arguments(inst)
        # execute the subprocess containing m4
        subprocess.run(m4_args)


if __name__ == '__main__':
    main()