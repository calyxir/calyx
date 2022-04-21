import json
import logging
import os
import sys
import random


def generate_data(template):
    with open(template) as f:
        template_file = json.load(f)
    for obj in template_file:
        data_var = template_file[obj]['data']
        format_var = template_file[obj]['format']
        if isinstance(data_var, list):
            result = revised_generate_list(data_var, format_var)
            construct_json(result, obj, template)
        elif isinstance(data_var, int):
            result = generate_int(format_var)
            construct_json(result, obj, template)
        else:
            raise TypeError


def revised_generate_list(input_data, data_format):
    try:
        # base case
        if isinstance(input_data, int):
            return generate_int(data_format)
        else:
            # recurrence
            for i in range(len(input_data)):
                input_data[i] = revised_generate_list(input_data[i], data_format)
        return input_data
    except Exception:
        raise TypeError


def generate_int(data_format):
    is_signed = data_format['is_signed']
    width = data_format['width']

    if is_signed:
        result = random.randrange(-2 ** (width - 1) + 1, 2 ** (width - 1) - 1)
    else:
        result = random.randrange(0, 2 ** width - 1)
    return result


def construct_json(data_list, obj, template):
    with open(template) as json_file:
        data = json.load(json_file)
    data[obj]["data"] = data_list

    with open(template, "w") as json_file:
        json.dump(data, json_file)


def compare_object(a, b):
    if type(a) != type(b):
        return False
    elif type(a) is dict:
        return compare_dict(a, b)
    elif type(a) is list:
        return compare_list(a, b)
    else:
        return a == b


def compare_dict(a, b):
    if len(a) != len(b):
        return False
    else:
        for k, v in a.items():
            if k not in b:
                return False
            else:
                if not compare_object(v, b[k]):
                    return False
    return True


def compare_list(a, b):
    if len(a) != len(b):
        return False
    else:
        for i in range(len(a)):
            if not compare_object(a[i], b[i]):
                return False
    return True


def do_fuzz(prog, spec, template, iteration):
    try:
        prog_name = os.path.splitext(prog)[0]
        prog_file_type = os.path.splitext(prog)[1]
        spec_name = os.path.splitext(spec)[0]
        spec_file_type = os.path.splitext(spec)[1]
        if prog_file_type != ".futil":
            prog = prog_name + ".futil"
            os.system(f"fud e {prog} --to futil > {prog}")

        elif spec_file_type != ".futil":
            spec = spec_name + ".futil"
            os.system(f"fud e {spec} --to futil > {spec}")

        for i in range(iteration):
            generate_data(template)
            if os.stat(template).st_size != 0:
                os.system(
                    f"fud e {prog} -s verilog.data {template} --to dat -q --through icarus-verilog > result1.json")
                os.system(
                    f"fud e {spec} -s verilog.data {template} --to dat -q --through icarus-verilog > result2.json")
                with open('result1.json', 'r') as f1, open('result2.json', 'r') as f2:
                    f1_result = json.load(f1)
                    f2_result = json.load(f2)
                    obj1 = f1_result['memories']
                    obj2 = f2_result['memories']
                    assert compare_object(obj1, obj2), "Failed with unequal output!"
    except Exception as e:
        logging.error(e)
        logging.error(f'fail unequal output for prog: {prog}, spec: {spec}')


if __name__ == "__main__":
    prog = sys.argv[1]
    spec = sys.argv[2]
    template = sys.argv[3]
    iteration = sys.argv[4]
    do_fuzz(prog, spec, template, int(iteration))
