import pathlib
import sys
import re
import json
from dataclasses import dataclass, asdict, is_dataclass
import argparse

toplevel: str = "main"


# Intermediate representation types
@dataclass
class CellWithParams:
    """
    Class representing a cell and its parameters.
    """

    cell_name: str
    cell_type: str
    cell_params: dict[str, int]


"""
Map from modules to cell names to cells with parameters.
"""
type ModuleCellTypes = dict[str, dict[str, CellWithParams]]

# Output representation types
"""
Map representing resources used by a cell.
"""
type Rsrc = dict[str, int]


@dataclass
class CellRsrc:
    """
    Class representing a cell and its resources.
    """

    cell_name: str
    cell_type: str
    cell_width: int | None
    generated: bool
    rsrc: Rsrc


"""
Map between qualified cell names and cell resource values.
"""
type DesignRsrc = dict[str, CellRsrc]


def parse_il_file_old(path: str) -> ModuleCellTypes:
    module_to_name_to_type: ModuleCellTypes = {}
    current_module = None
    with open(path, "r") as f:
        for line in f:
            line = line.strip()
            if line.startswith("module"):
                current_module = line.split()[1]
                module_to_name_to_type[current_module] = {}
            elif line.startswith("cell"):
                match = re.match(r"cell\s+(\S+)\s+(\S+)", line)
                if match:
                    cell_type, cell_name = match.groups()
                    module_to_name_to_type[current_module][cell_name] = cell_type
    return module_to_name_to_type


def parse_il_file(path: str) -> ModuleCellTypes:
    module_to_name_to_type: ModuleCellTypes = {}
    current_module = None
    current_cell = None
    with open(path, "r") as f:
        for line in f:
            line = line.strip()
            if line.startswith("module"):
                current_module = line.split()[1]
                module_to_name_to_type[current_module] = {}
            elif line.startswith("cell") and current_module:
                current_cell = line.split()[2]
                cell_type = line.split()[1]
                module_to_name_to_type[current_module][current_cell] = CellWithParams(
                    current_cell, cell_type, {}
                )
            elif line.startswith("parameter") and current_cell:
                param_name = line.split()[1]
                param_val = line.split()[2]
                module_to_name_to_type[current_module][current_cell].cell_params[
                    param_name
                ] = param_val
            elif line.startswith("end") and current_cell:
                current_cell = None
            elif line.startswith("end") and current_module:
                current_module = None
    return module_to_name_to_type


def flatten_il_rec_helper(
    module_to_name_to_type: ModuleCellTypes, module: str, pref: str
):
    design_map: DesignRsrc = {}
    for cell_name, cell_with_params in module_to_name_to_type[module].items():
        generated_type = cell_with_params.cell_type[0] == "$"
        generated_name = cell_name[0] == "$"
        if generated_type:
            width = max(
                {
                    int(v)
                    for k, v in cell_with_params.cell_params.items()
                    if k.endswith("WIDTH")
                },
                default=None,
            )
            if cell_with_params.cell_type.startswith("$paramod"):
                new_width = cell_with_params.cell_type.split("\\")[2]
                width = int(new_width.split("'")[1], 2)
            design_map[f"{pref}.{cell_name[1:]}"] = CellRsrc(
                cell_name[1:],
                cell_with_params.cell_type[1:],
                width,
                generated_name,
                {},
            )
        else:
            design_map |= flatten_il_rec_helper(
                module_to_name_to_type,
                cell_with_params.cell_type,
                f"{pref}.{cell_name[1:]}",
            )
    return design_map


def flatten_il(module_to_name_to_type: ModuleCellTypes):
    return flatten_il_rec_helper(module_to_name_to_type, "\\main", "main")


def parse_stat_file(path: str) -> dict:
    with open(path, "r") as f:
        return json.load(f)


def populate_stats(design_map: DesignRsrc, stat: dict):
    for k, v in design_map.items():
        if v.cell_type.startswith("paramod"):
            filtered_rsrc = {
                k: v
                for k, v in stat["modules"][f"${v.cell_type}"].items()
                if isinstance(v, int)
            }
            design_map[k].rsrc.update(filtered_rsrc)
            v.cell_type = v.cell_type.split("\\")[1]


def main():
    parser = argparse.ArgumentParser(
        description="Utility to process Yosys IL and stat files and dump design map as JSON"
    )
    parser.add_argument("il_file", type=pathlib.Path, help="path to the IL file")
    parser.add_argument("stat_file", type=pathlib.Path, help="path to the stat file")
    parser.add_argument(
        "-o",
        "--output",
        type=pathlib.Path,
        help="output JSON",
    )
    args = parser.parse_args()

    name_to_type = parse_il_file(args.il_file)
    design_map = flatten_il(name_to_type)
    stat = parse_stat_file(args.stat_file)
    populate_stats(design_map, stat)

    output_path = args.output

    if output_path:
        with open(output_path, "w") as f:
            json.dump(
                design_map,
                f,
                indent=2,
                default=lambda o: asdict(o) if is_dataclass(o) else str,
            )
    else:
        print(
            json.dumps(
                design_map,
                indent=2,
                default=lambda o: asdict(o) if is_dataclass(o) else str,
            )
        )


if __name__ == "__main__":
    main()
