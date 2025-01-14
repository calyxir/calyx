# a super simple script to show ADL level profiling proof of concept/experimentation
import json
import linecache
import os
import sys

def get_var_name(filename, linenum):
    line = linecache.getline(filename, linenum).strip()
    # let's see if there's a equals sign!!! I hate this!!!!
    if line.startswith("with") and ".group(" in line: # checking for groups that are declared via with. also janky. we need an actual parser
        varname = line.split(":")[0].split(" ")[-1]
    elif "=" in line:
        # janky as hell. probably should actually parse into Python and obtain the variable name but I want to see things work first
        before_equals = line.split("=")[0].strip()
        if before_equals.count(" ") != 0:
            varname = "unnamed"
        else:
            varname = before_equals
    else:
        varname = "unnamed"
    return varname

class SourceLoc:
    def __init__(self, filename, linenum):
        self.filename = filename
        self.linenum = linenum
        self.varname = get_var_name(filename, linenum)

    def __str__(self):
        return str((self.filename, str(self.linenum), self.varname))

    def json_repr(self):
        return {"filename": self.filename, "linenum": self.linenum, "varname": self.varname}

class Component:
    def __init__(self, name, pos_id):
        self.name = name
        self.position_id = pos_id
        self.position = None
        self.cells = {} # cell_name --> pos_id
        self.groups = {} # group_name --> pos_id

    def rewrite(self, position_map):
        # replace position IDs with the filename, linenum pair.
        self.position = position_map[self.position_id]
        for cell in self.cells:
            pos_id = self.cells[cell]
            self.cells[cell] = position_map[pos_id]
        for group in self.groups:
            pos_id = self.groups[group]
            self.groups[group] = position_map[pos_id]

    def __str__(self):
        s = f"Component {self.name}:\n"
        s += f"\tpos: {self.position_id}\n"
        s += f"\tcells:\n"
        for cell in self.cells:
            s += f"\t\t{cell}: {self.cells[cell]}\n"
        s += f"\tgroups:\n"
        for group in self.groups:
            s += f"\t\t{group}: {self.groups[group]}\n"
        return s

    def gen_dict_for_json(self):
        d = {"component" : self.name, "cells": [], "groups": []}
        d.update(self.position.json_repr()) # add position info
        for c in self.cells:
            cell_dict = {"name": c}
            cell_dict.update(self.cells[c].json_repr()) # add position info
            d["cells"].append(cell_dict)
        for g in self.groups:
            group_dict = {"name": g}
            group_dict.update(self.groups[g].json_repr())
            d["groups"].append(group_dict)
        return d

def parse(calyx_file):
    # a really hacky parser.
    metadata = False
    file_map = {}
    position_map = {}
    components = {} # name --> Component
    curr_component = None
    with open(calyx_file, "r") as r:
        for line in r:
            line_strip = line.strip()
            # start metadata
            if line_strip.startswith("metadata #{"):
                metadata = True
                continue
            if metadata:
                if line_strip.startswith("file-"):
                    line_split = line_strip.split("file-")[1].split(":")
                    file_id = line_split[0]
                    filename = line_split[1].strip()
                    file_map[file_id] = filename
                if line_strip.startswith("pos-"):
                    line_split = line_strip.split("pos-")[1].split(":")
                    position_id = line_split[0]
                    rest_split = line_split[1].strip().strip("(").strip(")").replace(" ", "").split(",")
                    file_id = rest_split[0]
                    line_num = int(rest_split[1])
                    position_map[position_id] = SourceLoc(file_map[file_id], line_num) # if the query fails sth went wrong
            else:
                # shoddy attempt at parsing an eDSL-generated Calyx file
                if line_strip.startswith("component"):
                    if curr_component is not None: # store the previous component
                        components[curr_component.name] = curr_component
                    name = line_strip.split(" ")[1].split("<")[0]
                    pos_id = line_strip.split('"pos"=')[1].split(">")[0]
                    curr_component = Component(name, pos_id)
                elif line_strip.startswith("group "):
                    groupname = line_strip.split("<")[0].split(" ")[-1]
                    pos_id = line_strip.split('"pos"=')[1].split(">")[0].split(",")[0]
                    if groupname in curr_component.groups:
                        raise RuntimeError(f"Group {groupname} already recorded in component {curr_component.name}")
                    curr_component.groups[groupname] = pos_id
                elif "@pos" in line_strip: # currently the other thing that has positions is cells. probably will change in the future
                    cellname = line_strip.split("=")[0].strip().split(" ")[-1]
                    pos_id = line_strip.split("@pos(")[1].split(")")[0]
                    if cellname in curr_component.cells:
                        raise RuntimeError(f"Cell {cellname} already recorded in component {curr_component.name}")
                    curr_component.cells[cellname] = pos_id
    # store last component
    components[curr_component.name] = curr_component

    return components, position_map

def main(calyx_file, out_file):
    components, position_map = parse(calyx_file)
    maps = []
    for component_name in components:
        component = components[component_name]
        component.rewrite(position_map)
        maps.append(component.gen_dict_for_json())

    with open(out_file, "w", encoding="utf-8") as out:
        out.write(json.dumps(maps, indent=4))

if __name__ == "__main__":
    if len(sys.argv) > 2:
        calyx_file = sys.argv[1]
        out_file = sys.argv[2]
        main(calyx_file, out_file)
    else:
        args_desc = [
            "CALYX_FILE",
            "OUT_JSON"
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        sys.exit(-1)
