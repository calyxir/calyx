import json
import logging
import os
import random
from deepdiff import DeepDiff
import argparse


def generate_data(template):
    """
    Randomly generate data based on the template
    """
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
    """
    Generate a list of data based on datatype contained in the list, data width, and sign.
    """
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
    """
    Generate an integer based on the given data width and sign.
    """
    is_signed = data_format['is_signed']
    width = data_format['width']

    if is_signed:
        result = random.randrange(-2 ** (width - 1) + 1, 2 ** (width - 1) - 1)
    else:
        result = random.randrange(0, 2 ** width - 1)
    return result


def construct_json(data_list, obj, template):
    """
    Insert the generated data to data template
    """
    with open(template) as json_file:
        data = json.load(json_file)
    data[obj]["data"] = data_list

    with open(template, "w") as json_file:
        json.dump(data, json_file)


def compare_object(a, b):
    """
    Compare input objects a and b. In this case, we only expect a and b to be either dictionary or list.
    """
    if type(a) != type(b):
        return False
    elif type(a) is dict:
        return compare_dict(a, b)
    elif type(a) is list:
        return compare_list(a, b)
    else:
        return a == b


def compare_dict(a, b):
    """
    Compare input dictionaries a and b.
    """
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
    """
    Compare input lists a and b.
    """
    if len(a) != len(b):
        return False
    else:
        for i in range(len(a)):
            if not compare_object(a[i], b[i]):
                return False
    return True


def do_fuzz_file(prog, spec, backend, template, iteration):
    """
    Compare two files. Run data generated from input template on both input file prog and spec.
    Raise exception if two files produce unequal results.
    Note: prog and spec do not need to be Calyx file, but should be convertable to Calyx using fud
    """

    try:
        print("Running do_fuzz_file!")
        original_prog = prog
        original_spec = spec
        prog_name, prog_file_type = os.path.splitext(prog)
        spec_name, spec_file_type = os.path.splitext(spec)
        if prog_file_type != ".futil":
            prog = prog_name + ".futil"
            os.system(f"fud e {original_prog} --to futil > {prog}")

        elif spec_file_type != ".futil":
            spec = spec_name + ".futil"
            os.system(f"fud e {original_spec} --to futil > {spec}")

        for i in range(iteration):
            generate_data(template)
            if os.stat(template).st_size == 0:
                continue
            os.system(
                f"fud e {prog} -s verilog.data {template} --to dat -q --through {backend} > result1.json")
            os.system(
                f"fud e {spec} -s verilog.data {template} --to dat -q --through {backend} > result2.json")
            with open('result1.json', 'r') as f1, open('result2.json', 'r') as f2:
                f1_result = json.load(f1)
                f2_result = json.load(f2)
                obj1 = f1_result['memories']
                obj2 = f2_result['memories']
                diff = DeepDiff(obj1, obj2)
                assert compare_object(obj1, obj2), f"Failed with unequal output! {diff}"


    except Exception as e:
        logging.error(e)
        logging.error(f'fail unequal output for prog: {prog}, spec: {spec}')


def do_fuzz_backend(prog, backend_1, backend_2, template, iteration):
    """
    Compare two backend tools. Run data generated from input template on same file using different backend tools.
    Raise exception if two backend tools produce unequal results.
    Note: file does not need to be Calyx file, but should be convertable to Calyx using fud. Backend tools should
    be recognizable by fud.
    """

    try:
        print("Running do_fuzz_backend!")
        original_prog = prog
        prog_name, prog_file_type = os.path.splitext(prog)
        if prog_file_type != ".futil":
            prog = prog_name + ".futil"
            os.system(f"fud e {original_prog} --to futil > {prog}")

        for i in range(iteration):
            generate_data(template)
            if os.stat(template).st_size == 0:
                continue
            os.system(
                f"fud e {prog} -s verilog.data {template} --to dat -q --through {backend_1} > result1.json")
            os.system(
                f"fud e {prog} -s verilog.data {template} --to dat -q --through {backend_2} > result2.json")
            with open('result1.json', 'r') as f1, open('result2.json', 'r') as f2:
                f1_result = json.load(f1)
                f2_result = json.load(f2)
                obj1 = f1_result['memories']
                obj2 = f2_result['memories']
                diff = DeepDiff(obj1, obj2)
                assert compare_object(obj1, obj2), f"Failed with unequal output! {diff}"

    except Exception as e:
        logging.error(e)
        logging.error(f'fail unequal output on file: {prog} with backend tools: {backend_1} and {backend_2}')


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run fuzzing on files.")
    parser.add_argument('--compare_backend', help='compare backend tools', nargs=5)
    parser.add_argument('--compare_files', help='compare two files', nargs=5)

    arg_lst = []
    for _, value in parser.parse_args()._get_kwargs():
        if value is not None:
            arg_lst = value

    if parser.parse_args().compare_files:
        do_fuzz_file(arg_lst[0], arg_lst[1], arg_lst[2], arg_lst[3], int(arg_lst[4]))
    elif parser.parse_args().compare_backend:
        do_fuzz_backend(arg_lst[2], arg_lst[0], arg_lst[1], arg_lst[3], int(arg_lst[4]))
