"""
Generates a .btor file for each module in each .sv file in the current directory
"""
import os
import json
import subprocess
import sys
import anytree
import tempfile
import verible_verilog_syntax


# Adapted from verible examples:
# https://github.com/chipsalliance/verible/blob/e76eb275b8e6739e9c9edc9e35032b193e0ce187/verilog/tools/syntax/export_json_examples/print_modules.py
def process_file_data(path: str, data: verible_verilog_syntax.SyntaxData):
    """Print information about modules found in SystemVerilog file.

    This function uses verible_verilog_syntax.Node methods to find module
    declarations and specific tokens containing following information:

    * module name
    * module port names
    * module parameter names
    * module imports
    * module header code

    Args:
      path: Path to source file (used only for informational purposes)
      data: Parsing results returned by one of VeribleVerilogSyntax' parse_*
            methods.
    """
    if not data.tree:
        return

    modules_info = []

    # Collect information about each module declaration in the file
    for module in data.tree.iter_find_all({"tag": "kModuleDeclaration"}):
        module_info = {
            "header_text": "",
            "name": "",
            "ports": [],
            "parameters": [],
            "imports": [],
        }

        # Find module header
        header = module.find({"tag": "kModuleHeader"})
        if not header:
            continue
        module_info["header_text"] = header.text

        # Find module name
        name = header.find(
            {"tag": ["SymbolIdentifier", "EscapedIdentifier"]},
            iter_=anytree.PreOrderIter,
        )
        if not name:
            continue
        module_info["name"] = name.text

        # Get the list of ports
        for port in header.iter_find_all({"tag": ["kPortDeclaration", "kPort"]}):
            port_id = port.find({"tag": ["SymbolIdentifier", "EscapedIdentifier"]})
            module_info["ports"].append(port_id.text)

        # Get the list of parameters
        for param in header.iter_find_all({"tag": ["kParamDeclaration"]}):
            param_id = param.find({"tag": ["SymbolIdentifier", "EscapedIdentifier"]})
            module_info["parameters"].append(param_id.text)

        # Get the list of imports
        for pkg in module.iter_find_all({"tag": ["kPackageImportItem"]}):
            module_info["imports"].append(pkg.text)

        modules_info.append(module_info)

    return modules_info


def gen_btor(yosys_executable, sv_filename, modules_info, out_dir="."):
    """
    Generates a .btor file for each module in the given .sv file
    """
    # create a temporary file (.ys) for the synthesis script
    _, synthesis_script_path = tempfile.mkstemp(suffix=".ys")

    # modify the synthesis script with a different prep -top for each module
    for module_info in modules_info:
        with open(synthesis_script_path, "w") as f:
            f.write(f"read -sv {sv_filename}\n")
            f.write(f"prep -top {module_info['name']}\n")
            f.write(
                f"write_btor -s {os.path.join(out_dir, module_info['name'])}.btor\n"
            )
        f.close()

        # print contents of synthesis script
        # with open(synthesis_script_path, "r") as f:
        #     print(f.read())

        # run yosys
        conversion_process = subprocess.Popen(
            [yosys_executable, synthesis_script_path],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        btor_out, btor_err = conversion_process.communicate()
        if btor_err:
            print(btor_err.decode("utf-8"))
            return 1


def main():
    if len(sys.argv) < 5:
        args_desc = [
            "PATH_TO_YOSYS_EXECUTABLE",
            "PATH_TO_VERIBLE_VERILOG_SYNTAX",
            "OUTPUT_DIR",
            "VERILOG_FILE [VERILOG_FILE [...]]",
        ]
        print(f"Usage: {sys.argv[0]} {'  '.join(args_desc)}")
        return 1

    yosys_path = sys.argv[1]
    parser_path = sys.argv[2]
    output_dir = sys.argv[3]
    file_paths = sys.argv[4:]

    # validate
    if not os.path.exists(yosys_path):
        print(f"Error: {yosys_path} does not exist")
        return 1
    if not os.path.exists(parser_path):
        print(f"Error: {parser_path} does not exist")
        return 1
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)

    # if the output directory is not empty, warn the user that it will be overwritten and confirm
    if os.listdir(output_dir):
        print(f"Warning: {output_dir} is not empty, and will be overwritten")
        print("Continue? [y/N]")
        if input().lower() != "y":
            return 1

        # clear the output directory
        for f in os.listdir(output_dir):
            os.remove(os.path.join(output_dir, f))

    # parse the files
    parser = verible_verilog_syntax.VeribleVerilogSyntax(executable=parser_path)
    data = parser.parse_files(file_paths)

    for file_path, file_data in data.items():
        modules_info = process_file_data(file_path, file_data)
        gen_btor(
            yosys_path,
            file_path,
            modules_info,
            output_dir,
        )


if __name__ == "__main__":
    sys.exit(main())
