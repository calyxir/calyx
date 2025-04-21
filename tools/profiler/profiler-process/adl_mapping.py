import json
import os


class SourceLoc:
    def __init__(self, json_dict):
        self.filename = os.path.basename(json_dict["filename"])
        self.linenum = json_dict["linenum"]
        self.varname = json_dict["varname"]

    def __repr__(self):
        return f"{self.filename}: {self.linenum}"


def read_adl_mapping_file(adl_mapping_file):
    component_mappings = {}  # component --> (filename, linenum)
    cell_mappings = {}  # component --> {cell --> (filename, linenum)}
    group_mappings = {}  # component --> {group --> (filename, linenum)}
    with open(adl_mapping_file, "r") as json_file:
        json_data = json.load(json_file)
    for component_dict in json_data:
        component_name = component_dict["component"]
        component_mappings[component_name] = SourceLoc(component_dict)
        cell_mappings[component_name] = {}
        for cell_dict in component_dict["cells"]:
            cell_mappings[component_name][cell_dict["name"]] = SourceLoc(cell_dict)
        # probably worth removing code clone at some point
        group_mappings[component_name] = {}
        for group_dict in component_dict["groups"]:
            group_mappings[component_name][group_dict["name"]] = SourceLoc(group_dict)
    return component_mappings, cell_mappings, group_mappings



def convert_flame_map(flame_map, adl_mapping_file):
    """
    Creates ADL and Mixed (ADL + Calyx) versions of flame graph maps.
    """    
    component_map, cell_map, group_map = read_adl_mapping_file(adl_mapping_file)
    adl_flame_map = {}
    mixed_flame_map = {}

    for stack in sorted(flame_map.keys()):
        cycles = flame_map[stack]
        adl_stack = []
        mixed_stack = []
        curr_component = None
        for stack_elem in stack.split(";"):
            # going to start by assuming "main" is the entrypoint.
            if stack_elem == "main":
                curr_component = stack_elem
                sourceloc = component_map[stack_elem]
                mixed_stack_elem = f"main {{{sourceloc}}}"
                adl_stack_elem = mixed_stack_elem
            elif "[" in stack_elem:  # invocation of component cell
                cell = stack_elem.split("[")[0].strip()
                cell_sourceloc = cell_map[curr_component][cell]
                cell_component = stack_elem.split("[")[1].split("]")[0]
                cell_component_sourceloc = component_map[cell_component]
                mixed_stack_elem = f"{cell} {{{cell_sourceloc}}} [{cell_component} {{{cell_component_sourceloc}}}]"
                adl_stack_elem = f"{cell_sourceloc.varname} {{{cell_sourceloc}}} [{cell_component_sourceloc.varname} {{{cell_component_sourceloc}}}]"
                curr_component = cell_component
            elif "(primitive)" in stack_elem:  # primitive
                primitive = stack_elem.split("(primitive)")[0].strip()
                primitive_sourceloc = cell_map[curr_component][primitive]
                mixed_stack_elem = f"{stack_elem} {{{primitive_sourceloc}}}"
                adl_stack_elem = (
                    f"{primitive_sourceloc.varname} {{{primitive_sourceloc}}}"
                )
            else:  # group
                # ignore compiler-generated groups (invokes) for now...
                if stack_elem in group_map[curr_component]:
                    sourceloc = group_map[curr_component][stack_elem]
                    adl_stack_elem = f"{sourceloc.varname} {{{sourceloc}}}"
                else:
                    sourceloc = "compiler-generated"
                    adl_stack_elem = sourceloc
                mixed_stack_elem = f"{stack_elem} {{{sourceloc}}}"
            adl_stack.append(adl_stack_elem)
            mixed_stack.append(mixed_stack_elem)
        # multiple Calyx stacks might have the same ADL stack (same source). If the ADL/mixed stack already exists in the map, we add the cycles from this Calyx stack.
        adl_stack_str = ";".join(adl_stack)
        mixed_stack_str = ";".join(mixed_stack)
        if adl_stack_str in adl_flame_map:
            adl_flame_map[adl_stack_str] += cycles
        else:
            adl_flame_map[adl_stack_str] = cycles
        if mixed_stack_str in mixed_flame_map:
            mixed_flame_map[mixed_stack_str] += cycles
        else:
            mixed_flame_map[mixed_stack_str] = cycles

    return adl_flame_map, mixed_flame_map
