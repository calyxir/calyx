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


def do_fuzz_file(args):
    """
    Compare two files. Run data generated from input template on both input file prog and spec.
    Raise exception if two files produce unequal results.
    Note: prog and spec do not need to be Calyx file, but should be convertable to Calyx using fud
    """

    try:
        print("Running do_fuzz_file!")

        original_prog = args.input_file_1
        original_spec = args.input_file_2
        prog_name, prog_file_type = os.path.splitext(original_prog)
        spec_name, spec_file_type = os.path.splitext(original_spec)
        if prog_file_type != ".futil":
            args.input_file_1 = prog_name + ".futil"
            os.system(f"fud e {original_prog} --to futil > {args.input_file_1}")

        elif spec_file_type != ".futil":
            args.input_file_2 = spec_name + ".futil"
            os.system(f"fud e {original_spec} --to futil > {args.input_file_2}")

        if not args.backend_tool:
            args.backend_tool = "icarus-verilog"
        if not args.iteration:
            args.iteration = 1

        for i in range(int(args.iteration)):
            generate_data(args.data_file)
            if os.stat(args.data_file).st_size == 0:
                continue
            os.system(
                f"fud e {args.input_file_1} -s verilog.data {args.data_file} --to dat -q --through {args.backend_tool} > result1.json")
            os.system(
                f"fud e {args.input_file_2} -s verilog.data {args.data_file} --to dat -q --through {args.backend_tool} > result2.json")
            with open('result1.json', 'r') as f1, open('result2.json', 'r') as f2:
                f1_result = json.load(f1)
                f2_result = json.load(f2)
                obj1 = f1_result['memories']
                obj2 = f2_result['memories']
                diff = DeepDiff(obj1, obj2)
                assert (obj1 == obj2), f"Failed with unequal output! {diff}"

    except Exception as e:
        logging.error(e)
        logging.error(f'fail unequal output for prog: {args.input_file_1}, spec: {args.input_file_2}')


def do_fuzz_backend(args):
    """
    Compare two backend tools. Run data generated from input template on same file using different backend tools.
    Raise exception if two backend tools produce unequal results.
    Note: file does not need to be Calyx file, but should be convertable to Calyx using fud. Backend tools should
    be recognizable by fud.
    """

    try:
        print("Running do_fuzz_backend!")
        original_prog = args.input_file
        prog_name, prog_file_type = os.path.splitext(original_prog)
        if prog_file_type != ".futil":
            args.input_file = prog_name + ".futil"
            os.system(f"fud e {original_prog} --to futil > {args.input_file}")

        if not args.backend_1:
            args.backend_tool = "icarus-verilog"
        if not args.iteration:
            args.iteration = 1

        for i in range(int(args.iteration)):
            generate_data(args.data_file)
            if os.stat(args.data_file).st_size == 0:
                continue
            os.system(
                f"fud e {args.input_file} -s verilog.data {args.data_file} --to jq --through {args.backend_1} --through dat -s "
                f"jq.expr '.memories' > result1.json")
            os.system(
                f"fud e {args.input_file} -s verilog.data {args.data_file} --to jq --through {args.backend_2} --through dat -s "
                f"jq.expr '.memories' > result2.json")
            with open('result1.json', 'r') as f1, open('result2.json', 'r') as f2:
                f1_result = json.load(f1)
                f2_result = json.load(f2)
                diff = DeepDiff(f1_result, f2_result)
                assert (f1_result == f2_result), f"Failed with unequal output! {diff}"

    except Exception as e:
        logging.error(e)
        logging.error(
            f'fail unequal output on file: {args.input_file} with backend tools: {args.backend_1} and {args.backend_2}')


def conf_check_file(parser):
    parser.add_argument('-input_1', dest="input_file_1", help="Path to the input file 1", nargs="?")
    parser.add_argument('-input_2', dest="input_file_2", help="Path to the input file 2", nargs="?")

    parser.add_argument('-backend', dest='backend_tool', help="Receive the backend tool; use icarus0-verilog as "
                                                              "default")
    parser.add_argument('-dat', dest="data_file", help="Receive data file")

    parser.add_argument('-itr', dest="iteration", help="Number of iterations")

    parser.set_defaults(command='file')


def conf_check_backend(parser):
    parser.add_argument("-input", dest="input_file", help="Path to the input file", nargs="?")

    parser.add_argument('-backend_1', dest='backend_1', help="Receive the first backend tool")
    parser.add_argument('-backend_2', dest='backend_2', help="Receive the second backend tool")

    parser.add_argument('-dat', dest="data_file", help="Receive data file")

    parser.add_argument('-itr', dest="iteration", help="Number of iterations")

    parser.set_defaults(command='backend')


if __name__ == "__main__":
    p = argparse.ArgumentParser(description="Run fuzzing on files and backends.")
    sp = p.add_subparsers()

    conf_check_file(sp.add_parser('file', help="Run fuzz to compare two input files."))
    conf_check_backend(sp.add_parser('backend', help="Run fuzz to compare two backend tools."))

    args = p.parse_args()

    if "command" not in args:
        p.print_help()
        exit(-1)

    try:
        if args.command == 'file':
            if not (args.input_file_1 or args.input_file_2):
                p.error("Please provide two files to compare")
            elif not args.data_file:
                p.error("Please provide a data template file as reference")
            else:
                do_fuzz_file(args)
        elif args.command == 'backend':
            if not args.input_file:
                p.error("Please provide a file as reference")
            elif not (args.backend_1 or args.backend_2):
                p.error("Please provide two backend tools to compare")
            elif not args.data_file:
                p.error("Please provide a data template file as reference")
            else:
                do_fuzz_backend(args)
    except Exception as e:
        logging.error(e)
        exit(-1)
