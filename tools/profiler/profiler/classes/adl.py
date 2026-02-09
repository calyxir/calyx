from functools import reduce
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
    # adl line num --> line content
    adl_linum_map: dict[int, str]
    # source ADL
    adl: Adl

    def __init__(self, adl_mapping_file: str):
        self.component_map = {}
        self.cell_map = {}
        self.group_map = {}
        self.adl_linum_map = {}
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
                component_sourceloc = self._read_entry(component_dict)
                self.component_map[component_name] = component_sourceloc
                self.cell_map[component_name] = {}
                for cell_dict in component_dict["cells"]:
                    self.cell_map[component_name][cell_dict["name"]] = self._read_entry(
                        cell_dict
                    )
                # probably worth removing code clone at some point
                self.group_map[component_name] = {}
                for group_dict in component_dict["groups"]:
                    self.group_map[component_name][group_dict["name"]] = (
                        self._read_entry(group_dict)
                    )

    def _read_entry(self, entry_dict):
        """
        Helper function for creating a SourceLoc object and registering to adl_linum_map
        the contents of a source location.
        Returns the SourceLoc object created.
        """
        sourceloc = SourceLoc(entry_dict)
        if sourceloc.linenum is not None:
            # FIXME: HARDCODED TO LINE UP WITH THE DAHLIA TRACE THIS IS NOT OK
            # Note to self: test Calyx-py as well.
            self.adl_linum_map[sourceloc.linenum] = (
                f"L{sourceloc.linenum:04}: {sourceloc.varname}"
            )
        return sourceloc


@dataclass
class DahliaAdlMap(AdlMap):
    """
    An AdlMap for Dahlia containing extra metadata for processing
    Dahlia traces. Specifically, this class contains a set of all
    blocks (for loops, while loops, and if condtionals), and a map
    from statements to their block ancestors (blocks that are active)
    when the statement is active.
    """

    # statement --> [b1, b2, ...] where b1 is the immediate parent
    stmt_to_block_ancestors: dict[str, list[str]]
    # names of all blocks
    blocks: set[str]

    def __init__(self, adl_map: AdlMap, dahlia_block_map: str | None):
        # copy the original ADL map bit? Maybe this will be unnecessary
        self.component_map = adl_map.component_map
        self.cell_map = adl_map.cell_map
        self.group_map = adl_map.group_map
        self.adl_linum_map = adl_map.adl_linum_map
        self.adl = Adl.DAHLIA
        # initialize Dahlia-specific part.
        self.stmt_to_block_ancestors, self.blocks = self._process_dahlia_parent_map(
            dahlia_block_map
        )

    @staticmethod
    def block_name(line_contents: str):
        block_prefix = "B"
        return f"{block_prefix}{line_contents}"

    @staticmethod
    def _read_json_parent_map(parent_map_file):
        """
        Helper function for _process_dahlia_parent_map()
        JSON is annoying and requires string keys. This function returns a map obtained from parent_map_file, but with int keys instead.
        """
        m = json.load(open(parent_map_file))
        return {int(k): m[k] for k in m}

    def _process_dahlia_parent_map(self, dahlia_block_map: str | None):
        """
        Sets up parent-child relationships between Dahlia statements and blocks. Returns:
        - statement_to_block_ancestors: statement --> [b1, b2, ...] where b1 is the immediate parent (b1, b2, .. are in increasing order)
        - blocks: a set of all blocks that are in the program
        """
        statement_to_block_ancestors: dict[str, list[str]] = {}
        if dahlia_block_map is None:
            # return default dictionary (every line has zero block ancestors) and set.
            print(
                "dahlia_parent_map was not given; somewhat inconvenient timeline view will be generated"
            )
            return {
                self.adl_linum_map[linum]: [] for linum in self.adl_linum_map
            }, set()

        json_parent_map: dict[int, list[int]] = self._read_json_parent_map(
            dahlia_block_map
        )

        # need to have a parent block version of each one
        all_block_linums: set[int] = reduce(
            (lambda l1, l2: set(l1).union(set(l2))), json_parent_map.values()
        )

        linum_to_block = {
            linum: self.block_name(self.adl_linum_map[linum])
            for linum in all_block_linums
        }

        # figure out child-parent mappings.
        for linum in sorted(json_parent_map, key=(lambda x: len(json_parent_map[x]))):
            line_contents = self.adl_linum_map[linum]

            # identify the immediate ancestor trackids
            if linum in all_block_linums and len(json_parent_map[linum]) == 0:
                # this line is a parent line with no parents of its own,
                # the parent is the block version of this line.
                block_track_id = self.block_name(line_contents)
                statement_to_block_ancestors[line_contents] = [block_track_id]
                # block version also gets added?
                statement_to_block_ancestors[block_track_id] = []

            elif linum in all_block_linums:
                # this line is a parent line that itself has parents
                block_track_id = self.block_name(line_contents)
                ancestor_list = list(
                    map((lambda p: linum_to_block[p]), json_parent_map[linum])
                )

                # this line's parent is the block version of this line.
                statement_to_block_ancestors[line_contents] = [
                    block_track_id
                ] + ancestor_list

                # the block version of this line's ancestors are the block version of the actual ancestors of this line.
                statement_to_block_ancestors[block_track_id] = ancestor_list

            elif len(json_parent_map[linum]) > 0:
                # this line is a "normal" line with ancestors.
                # use block version of the actual ancestors.
                ancestor_list = list(
                    map(
                        (lambda p: self.block_name(self.adl_linum_map[p])),
                        json_parent_map[linum],
                    )
                )

                statement_to_block_ancestors[line_contents] = ancestor_list

            else:
                # otherwise is a "normal" line with NO parents.
                statement_to_block_ancestors[line_contents] = []

        return statement_to_block_ancestors, set(linum_to_block.values())
