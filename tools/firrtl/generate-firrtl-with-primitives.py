import json
import os
import subprocess
import sys

# Generates the arguments for m4 based on the JSON data
def generate_m4_arguments(inst):
    primitive_name = inst["name"]
    args = []
    # hack to replace the module name with the corresponding parameterized version
    # FIXME: figure out a way to do substring replacement in m4
    module_name_value = primitive_name
    # get the parameters for the primitive
    for param in inst["params"]:
        key = param["param_name"]
        value = param["param_value"]
        args.append(f"-D{key}={value}")
        module_name_value += "_" + str(value)

    args.append(f"-DMODULE_NAME={module_name_value}")

    # retrieve the appropriate template file for the primitive
    firrtl_file_path = os.path.join(sys.path[0], "templates", primitive_name + ".fir")
    if not(os.path.isfile(firrtl_file_path)):
        print(f"{sys.argv[0]}: FIRRTL template file for primitive {primitive_name} does not exist! Exiting...")
        sys.exit(1)
    args.append(firrtl_file_path)

    return args

def generate(firrtl_filename, primitive_uses_filename):
    firrtl_file = open(firrtl_filename)
    primitive_uses_file = open(primitive_uses_filename)
    # The first line contains the circuit name, which needs to come before the primitives.
    print(firrtl_file.readline().rstrip())
    # Display the primitive definitions.
    primitive_insts = json.load(primitive_uses_file)
    if primitive_insts:
        for inst in primitive_insts:
            m4_args = ["m4"]
            m4_args += generate_m4_arguments(inst)
            p = subprocess.run(m4_args, capture_output=True)
            print(str(p.stdout.decode()))
    # Display the rest of the FIRRTL program.
    for line in firrtl_file.readlines():
        print(line.rstrip())


def main():
    if len(sys.argv) != 3:
        args_desc = [                                                                                                                                                                   
            "FIRRTL_FILE",
            "PRIMITIVE_USES_JSON"
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")                                                                                                                           
        return 1
    generate(sys.argv[1], sys.argv[2])

if __name__ == '__main__':
        main()