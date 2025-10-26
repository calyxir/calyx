import re
import json
from dataclasses import dataclass
from AreaExtract.lib.cdf.cdf import YosysRsrc, Cell, Design, Metadata, DesignWithMetadata

toplevel: str = "main"

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
    design_map: Design = {}
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
            design_map[f"{pref}.{cell_name[1:]}"] = Cell(
                cell_name[1:],
                cell_with_params.cell_type[1:],
                generated_name,
                {"width": width},
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


def populate_stats(design_map: Design, stat: dict):
    for k, v in design_map.items():
        if v.type.startswith("paramod"):
            filtered_rsrc: YosysRsrc = {
                k: v
                for k, v in stat["modules"][f"${v.type}"].items()
                if isinstance(v, int)
            }
            design_map[k].rsrc.update(filtered_rsrc)
            v.type = v.type.split("\\")[1]


def il_to_design_with_metadata(il_path: str, stat_path: str) -> DesignWithMetadata:
    modules = parse_il_file(il_path)
    design = flatten_il(modules)
    stat = parse_stat_file(stat_path)
    populate_stats(design, stat)
    return DesignWithMetadata(design=design, metadata=Metadata(origin="Yosys"))
