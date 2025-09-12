import os
import copy
import json
from collections import defaultdict
from dataclasses import dataclass, field
from enum import Enum
import statistics




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

class ParChildType(Enum):
    GROUP = 1
    PAR = 2


@dataclass(frozen=True)
class ParChildInfo:
    child_name: str
    child_type: ParChildType
    parents: set[str] = field(default_factory=set)

    def register_new_parent(self, new_parent: str):
        # FIXME: deprecate this method and instead obtain the entire parent set upfront.
        self.parents.add(new_parent)




class FlameMapMode(Enum):
    CALYX = 1
    ADL = 2
    MIXED = 3


@dataclass
class GroupSummary:
    """
    Summary for groups on the number of times they were active vs their active cycles
    """

    display_name: str
    num_times_active: int = 0
    active_cycles: set[int] = field(default_factory=set)

    interval_lengths: list[int] = field(default_factory=list)

    def register_interval(self, interval: range):
        self.num_times_active += 1
        self.active_cycles.update(set(interval))
        self.interval_lengths.append(len(interval))

    def fieldnames():
        return [
            "group-name",
            "num-times-active",
            "total-cycles",
            "min",
            "max",
            "avg",
            "can-static",
        ]

    def stats(self):
        stats = {}
        stats["group-name"] = self.display_name
        stats["num-times-active"] = self.num_times_active
        stats["total-cycles"] = len(self.active_cycles)
        min_interval = min(self.interval_lengths)
        max_interval = max(self.interval_lengths)
        avg_interval = round(statistics.mean(self.interval_lengths), 1)
        stats["min"] = min_interval
        stats["max"] = max_interval
        stats["avg"] = avg_interval
        stats["can-static"] = "Y" if min_interval == max_interval else "N"
        return stats


@dataclass
class Summary:
    """
    Summary for Cells/Control groups on the number of times they were active vs their active cycles
    FIXME: Add min/max/avg and collect these for normal groups as well?
    """

    num_times_active: int = 0
    active_cycles: set[int] = field(default_factory=set)


class ControlRegUpdateType(Enum):
    FSM = 1
    PAR_DONE = 2
    BOTH = 3


@dataclass(frozen=True)
class ControlRegUpdates:
    """
    Updates to control registers in a cell.
    Retain this info to add to the timeline
    """

    cell_name: str
    clock_cycle: int
    updates: str
