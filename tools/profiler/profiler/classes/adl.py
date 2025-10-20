import json
import os
from dataclasses import dataclass
from enum import Enum
from .errors import ProfilerException


class Adl(Enum):
    DAHLIA = 1
    PY = 2


@dataclass
class SourceLoc:
    """
    ADL source location information obtained from metadata.
    """

    filename: str | None
    linenum: int | None
    varname: str | None

    def __init__(self, json_dict):
        self.filename = (
            os.path.basename(json_dict["filename"])
            if json_dict["filename"] is not None
            else None
        )
        self.linenum = json_dict["linenum"]
        varname = json_dict["varname"]
        if varname is not None:
            self.varname = varname.replace(";", "").replace("{", "")
        else:
            self.varname = None

    def adl_str(self):
        return f"{{{self.filename}: {self.linenum}}} {self.varname}"

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
    # source ADL
    adl: Adl

    def __init__(self, adl_mapping_file: str):
        self.component_map = {}
        self.cell_map = {}
        self.group_map = {}
        with open(adl_mapping_file, "r") as json_file:
            json_data = json.load(json_file)
            if json_data["adl"] == "Dahlia":
                self.adl = Adl.DAHLIA
            elif json_data["adl"] == "Py":
                self.adl = Adl.PY
            else:
                raise ProfilerException(f"Unimplemented ADL {json_data['adl']}")
            for component_dict in json_data["components"]:
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
