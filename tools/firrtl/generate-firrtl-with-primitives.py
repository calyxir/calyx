import json
import math
import os
import sys

# Generates a map where `key` should be replaced with `value`
def generate_replacement_map(inst):
    replacement_map = {}
    for param in inst["params"]:
        replacement_map[param["param_name"]] = param["param_value"]

    # Special primitives that have a value dependent on their parameters.
    if inst["name"] == "std_pad":
        replacement_map["DIFF"] = replacement_map["OUT_WIDTH"] - replacement_map["IN_WIDTH"]
    elif inst["name"] == "std_slice":
        replacement_map["DIFF"] = replacement_map["IN_WIDTH"] - replacement_map["OUT_WIDTH"]
    elif inst["name"] == "std_lsh":
        width = replacement_map["WIDTH"]
        replacement_map["BITS"] = math.ceil(math.log(width, 2)) + 1
    elif inst["name"] == "std_mult_pipe":
        width = replacement_map["WIDTH"]
        replacement_map["W_SHIFTED_ONE"] = width << 1
        replacement_map["HIGH"] = width - 1
        replacement_map["LOW"] = 0


    return replacement_map

# Retrieves the appropriate template file for the given primitive
def retrieve_firrtl_template(primitive_name):
    firrtl_file_path = os.path.join(sys.path[0], "templates", primitive_name + ".fir")
    if not(os.path.isfile(firrtl_file_path)):
        print(f"{sys.argv[0]}: FIRRTL template file for primitive {primitive_name} does not exist! Exiting...")
        sys.exit(1)
    return firrtl_file_path

# Generates a primitive definition from the provided JSON data of a unique primitive use
def generate_primitive_definition(inst):
    template_filename = retrieve_firrtl_template(inst["name"])
    replacement_map = generate_replacement_map(inst)

    with open(template_filename, "r") as template_file:
        for line in template_file:
            for key in replacement_map:
                line = line.replace(key, str(replacement_map[key]))
            print(line.rstrip())
    print() # whitespace to buffer between modules

# Generates a complete FIRRTL program with primitives.
def generate(firrtl_filename, primitive_uses_filename):
    firrtl_file = open(firrtl_filename)
    primitive_uses_file = open(primitive_uses_filename)
    # The first line contains the circuit name, which needs to come before the primitives.
    print(firrtl_file.readline().rstrip())
    # Display the primitive definitions.
    primitive_insts = json.load(primitive_uses_file)
    if primitive_insts:
        for inst in primitive_insts:
            generate_primitive_definition(inst)
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
