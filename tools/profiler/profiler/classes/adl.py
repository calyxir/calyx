import json
import os
from dataclasses import dataclass


@dataclass
class SourceLoc:
    """
    ADL source location information obtained from metadata.
    """

    filename: str
    linenum: int
    varname: str

    def __init__(self, json_dict):
        self.filename = os.path.basename(json_dict["filename"])
        self.linenum = json_dict["linenum"]
        self.varname = json_dict["varname"]

    def adl_str(self):
        return f"{self.varname} {{{self.filename}: {self.linenum}}}"

    def loc_str(self):
        return f"{{{self.filename}: {self.linenum}}}"


@dataclass
class AdlMap:
    """
    Mappings from Calyx components, cells, and groups to the corresponding ADL SourceLoc.
    """

    # component --> (filename, linenum)
    component_map: dict[str, SourceLoc]
    # component --> {cell --> (filename, linenum)}
    cell_map: dict[str, dict[str, SourceLoc]]
    # component --> {group --> (filename, linenum)}
    group_map: dict[str, dict[str, SourceLoc]]

    def __init__(self, adl_mapping_file: str):
        self.component_map = {}
        self.cell_map = {}
        self.group_map = {}
        with open(adl_mapping_file, "r") as json_file:
            json_data = json.load(json_file)
            for component_dict in json_data:
                component_name = component_dict["component"]
                self.component_map[component_name] = SourceLoc(component_dict)
                self.cell_map[component_name] = {}
                for cell_dict in component_dict["cells"]:
                    self.cell_map[component_name][cell_dict["name"]] = SourceLoc(
                        cell_dict
                    )
                # probably worth removing code clone at some point
                self.group_map[component_name] = {}
                for group_dict in component_dict["groups"]:
                    self.group_map[component_name][group_dict["name"]] = SourceLoc(
                        group_dict
                    )
