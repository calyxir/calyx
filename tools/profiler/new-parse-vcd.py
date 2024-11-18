import csv
import json
import os
import re
import shutil
import sys
import vcdvcd

DELIMITER = "__"
INVISIBLE = "gray"

def remove_size_from_name(name: str) -> str:
    """ changes e.g. "state[2:0]" to "state" """
    return name.split('[')[0]


class ProfilingInfo:
    def __init__(self, name, callsite=None, component=None, is_cell=False):
        self.name = name
        self.callsite = callsite # "official" call site. Should only be used for probes?
        self.component = component
        self.shortname = self.name.split(".")[-1]
        self.closed_segments = [] # Segments will be (start_time, end_time)
        self.current_segment = None
        self.total_cycles = 0
        self.is_cell = is_cell
    
    def flame_repr(self):
        if self.is_cell:
            return self.name
        else:
            return self.shortname

    def __repr__ (self):
        if self.is_cell:
            header = f"[Cell] {self.name}" # FIXME: fix this later
        else:
            header = f"[{self.component}][{self.callsite}] {self.name}"
        return header

    def nice_repr (self):
        segments_str = ""
        for segment in self.closed_segments:
            if (segments_str != ""):
                segments_str += ", "
            segments_str += f"[{segment['start']}, {segment['end']})"
        if self.is_cell:
            header = f"[Cell] {self.name}\n" # FIXME: fix this later
        else:
            header = f"[{self.component}][{self.callsite}] {self.name}\n"
        return (header +
        f"\tTotal cycles: {self.total_cycles}\n" +
        f"\t# of times active: {len(self.closed_segments)}\n" +
        f"\tSegments: {segments_str}\n"
        )    

    def start_new_segment(self, curr_clock_cycle):
        if self.current_segment is None:
            self.current_segment = {"start": curr_clock_cycle, "end": -1, "callsite": self.callsite} # NOTE: see if this backfires
        else:
            print(f"Error! The group {self.name} is starting a new segment while the current segment is not closed.")
            print(f"Current segment: {self.current_segment}")
            sys.exit(1)

    def end_current_segment(self, curr_clock_cycle):
        if self.current_segment is not None and self.current_segment["end"] == -1: # ignore cases where done is high forever
            self.current_segment["end"] = curr_clock_cycle
            self.closed_segments.append(self.current_segment)
            self.total_cycles += curr_clock_cycle - self.current_segment["start"]
            self.current_segment = None # Reset current segment

    def get_segment_active_at_cycle(self, i): # get the segment that contains cycle i if one exists. Otherwise return None
        for segment in self.closed_segments:
            if segment["start"] <= i and i < segment["end"]:
                return segment
        return None

    def is_active_at_cycle(self, i): # is the probe active at cycle i?
        return self.get_segment_active_at_cycle(i) != None        

    def id(self):
        return f"{self.name}{DELIMITER}{self.component}"

class VCDConverter(vcdvcd.StreamParserCallbacks):
    def __init__(self, main_component, cells_to_components):
        # NOTE: assuming single-component programs for now
        super().__init__()
        self.main_component = main_component
        self.probes = set()
        # Map from timestamps [ns] to value change events that happened on that timestamp
        self.timestamps_to_events = {}
        self.cells = cells_to_components
        self.active_elements_info = {} # for every group/cell, maps to a corresponding ProfilingInfo object signalling when group/cell was active
        self.call_stack_probe_info = {} # group --> {parent group --> ProfilingInfo object}. This is for tracking structural enables within the same component.
        self.cell_invoke_probe_info = {} # {cell}_{component the cell was called from} --> {parent group --> ProfilingInfo Object}  (FIXME: maybe we want this the other way around?)
        self.cell_invoke_caller_probe_info = {} # {group}_{component} --> {cell --> ProfilingInfo object}
        for cell in self.cells:
            self.active_elements_info[cell] = ProfilingInfo(cell, is_cell=True)

    def enddefinitions(self, vcd, signals, cur_sig_vals):
        # convert references to list and sort by name
        refs = [(k, v) for k, v in vcd.references_to_ids.items()]
        refs = sorted(refs, key=lambda e: e[0])
        names = [remove_size_from_name(e[0]) for e in refs]
        signal_id_dict = {sid : [] for sid in vcd.references_to_ids.values()} # one id can map to multiple signal names since wires are connected

        clock_name = f"{self.main_component}.clk"
        if clock_name not in names:
            print("Can't find the clock? Exiting...")
            sys.exit(1)
        signal_id_dict[vcd.references_to_ids[clock_name]] = [clock_name]

        # get go and done for cells (the signals are exactly {cell}.go and {cell}.done)
        for cell in self.cells:
            # FIXME: check if anything here is different when we go over multicomponent programs
            cell_go = cell + ".go"
            cell_done = cell + ".done"
            if cell_go not in vcd.references_to_ids:
                print(f"Not accounting for cell {cell} (probably combinational)")
                continue
            signal_id_dict[vcd.references_to_ids[cell_go]].append(cell_go)
            signal_id_dict[vcd.references_to_ids[cell_done]].append(cell_done)

        for name, sid in refs:
            # check if we have a probe. instrumentation probes are "<group>__<callsite>__<component>_probe".
            # callsite is either the name of another group or "instrumentation_wrapper[0-9]*" in the case it is invoked from control.
            if "group_probe_out" in name: # this signal is a probe for group activation.
                encoded_info = name.split("_group_probe_out")[0]
                probe_info_split = encoded_info.split("__")
                group_name = probe_info_split[0]
                group_component = probe_info_split[1]
                self.active_elements_info[encoded_info] = ProfilingInfo(group_name, component=group_component)
                signal_id_dict[sid].append(name)

            elif "se_probe_out" in name: # this signal is a probe for structural enables.
                encoded_info = name.split("_se_probe_out")[0]
                probe_info_split = encoded_info.split("__")
                group_name = probe_info_split[0]
                group_parent = probe_info_split[1]
                group_component = probe_info_split[2]
                group_id = group_name + DELIMITER + group_component
                if group_id not in self.call_stack_probe_info:
                    self.call_stack_probe_info[group_id] = {}
                self.call_stack_probe_info[group_id][group_parent] = ProfilingInfo(group_name, callsite=group_parent, component=group_component)
                signal_id_dict[sid].append(name)

            elif "cell_probe_out" in name:
                encoded_info = name.split("_cell_probe_out")[0]
                probe_info_split = encoded_info.split("__")
                cell_name = probe_info_split[0]
                invoker_group = probe_info_split[1]
                component = probe_info_split[2]
                probe_info_obj = ProfilingInfo(cell_name, callsite=invoker_group, component=component, is_cell=True)
                cell_id = cell_name + DELIMITER + component
                if cell_id not in self.cell_invoke_probe_info:
                    self.cell_invoke_probe_info[cell_id] = {invoker_group: probe_info_obj}
                else:
                    self.call_stack_probe_info[cell_id][invoker_group] = probe_info_obj
                caller_id = invoker_group + DELIMITER + component
                if caller_id not in self.cell_invoke_caller_probe_info:
                    self.cell_invoke_caller_probe_info[caller_id] = {cell_name : probe_info_obj}
                else:
                    self.cell_invoke_caller_probe_info[caller_id][cell_name] = probe_info_obj
                signal_id_dict[sid].append(name)

        # don't need to check for signal ids that don't pertain to signals we're interested in
        self.signal_id_to_names = {k:v for k,v in signal_id_dict.items() if len(v) > 0}
    
    def value(self, vcd, time, value, identifier_code, cur_sig_vals):
        # ignore all signals we don't care about
        if identifier_code not in self.signal_id_to_names:
            return
        
        signal_names = self.signal_id_to_names[identifier_code]
        int_value = int(value, 2)

        if time not in self.timestamps_to_events:
            self.timestamps_to_events[time] = []

        for signal_name in signal_names:
            event = {"signal": signal_name, "value": int_value}
            self.timestamps_to_events[time].append(event)
    
    # Postprocess data mapping timestamps to events (signal changes)
    # We have to postprocess instead of processing signals in a stream because
    # signal changes that happen at the same time as a clock tick might be recorded
    # *before* or *after* the clock change on the VCD file (hence why we can't process
    # everything within a stream if we wanted to be precise)
    def postprocess(self):
        clock_name = f"{self.main_component}.clk"
        clock_cycles = -1
        started = False
        currently_active = set()
        se_currently_active = set() # structural group enables
        ci_currently_active = set() # cell invokes
        for ts in self.timestamps_to_events:
            events = self.timestamps_to_events[ts]
            started = started or [x for x in events if x["signal"] == f"{self.main_component}.go" and x["value"] == 1]
            if not started: # only start counting when main component is on.
                continue
            # checking whether the timestamp has a rising edge (hacky)
            if {"signal": clock_name, "value": 1} in events:
                clock_cycles += 1
            for event in events:
                signal_name = event["signal"]
                value = event["value"]
                if signal_name.endswith(".go") and value == 1: # cells have .go and .done
                    cell = signal_name.split(".go")[0]
                    self.active_elements_info[cell].start_new_segment(clock_cycles)
                    currently_active.add(cell)
                if signal_name.endswith(".done") and value == 1: # cells have .go and .done
                    cell = signal_name.split(".done")[0]
                    self.active_elements_info[cell].end_current_segment(clock_cycles)
                    currently_active.remove(cell)
                if "group_probe_out" in signal_name and value == 1: # instrumented group started being active
                    encoded_info = signal_name.split("_group_probe_out")[0]
                    self.active_elements_info[encoded_info].start_new_segment(clock_cycles)
                    currently_active.add(encoded_info)
                elif "group_probe_out" in signal_name and value == 0: # instrumented group stopped being active
                    encoded_info = signal_name.split("_group_probe_out")[0]
                    self.active_elements_info[encoded_info].end_current_segment(clock_cycles)
                    currently_active.remove(encoded_info)
                elif "se_probe_out" in signal_name and value == 1:
                    encoded_info_split = signal_name.split("_se_probe_out")[0].split("__")
                    child_group_name = encoded_info_split[0]
                    parent = encoded_info_split[1]
                    child_group_component = encoded_info_split[2]
                    group_id = child_group_name + DELIMITER + child_group_component
                    self.call_stack_probe_info[group_id][parent].start_new_segment(clock_cycles)
                    se_currently_active.add(group_id)
                elif "se_probe_out" in signal_name and value == 0:
                    encoded_info_split = signal_name.split("_se_probe_out")[0].split("__")
                    child_group_name = encoded_info_split[0]
                    parent = encoded_info_split[1]
                    child_group_component = encoded_info_split[2]
                    group_id = child_group_name + DELIMITER + child_group_component
                    self.call_stack_probe_info[group_id][parent].end_current_segment(clock_cycles)
                    se_currently_active.remove(group_id)
                elif "cell_probe_out" in signal_name and value == 1:
                    encoded_info_split = signal_name.split("_cell_probe_out")[0].split("__")
                    cell_name = encoded_info_split[0]
                    parent = encoded_info_split[1]
                    parent_component = encoded_info_split[2]
                    caller_id = parent + DELIMITER + parent_component
                    self.cell_invoke_caller_probe_info[caller_id][cell_name].start_new_segment(clock_cycles)
                    ci_currently_active.add(caller_id)
                    # cell_id = cell_name + DELIMITER + parent_component
                    # self.cell_invoke_probe_info[cell_id][parent].start_new_segment(clock_cycles)
                elif "cell_probe_out" in signal_name and value == 0:
                    encoded_info_split = signal_name.split("_cell_probe_out")[0].split("__")
                    cell_name = encoded_info_split[0]
                    parent = encoded_info_split[1]
                    parent_component = encoded_info_split[2]
                    caller_id = parent + DELIMITER + parent_component
                    self.cell_invoke_caller_probe_info[caller_id][cell_name].end_current_segment(clock_cycles)
                    ci_currently_active.remove(caller_id)
                    # cell_id = cell_name + DELIMITER + parent_component
                    # self.cell_invoke_probe_info[cell_id][parent].end_current_segment(clock_cycles)
        for active in currently_active: # end any group/cell activitations that are still around...
            self.active_elements_info[active].end_current_segment(clock_cycles)
        # FIXME: pretty sure the next two blocks fail because both stack infos are nested dictionaries lmao
        for active in se_currently_active: # end any structural enables that are still around...
            self.call_stack_probe_info[active].end_current_segment(clock_cycles)
        for active in ci_currently_active:
            self.cell_invoke_caller_probe_info[active].end_current_segment(clock_cycles)

        self.clock_cycles = clock_cycles

# Generates a list of all of the components to potential cell names
# `prefix` is the cell's "path" (ex. for a cell "my_cell" defined in "main", the prefix would be "TOP.toplevel.main")
# The initial value of curr_component should be the top level/main component
def build_components_to_cells(prefix, curr_component, cells_to_components, components_to_cells):
    for (cell, cell_component) in cells_to_components[curr_component].items():
        if cell_component not in components_to_cells:
            components_to_cells[cell_component] = [f"{prefix}.{cell}"]
        else:
            components_to_cells[cell_component].append(f"{prefix}.{cell}")
        build_components_to_cells(prefix + f".{cell}", cell_component, cells_to_components, components_to_cells)

# Reads json generated by component-cells backend to produce a mapping from all components
# to cell names they could have.
def read_component_cell_names_json(json_file):
    cell_json = json.load(open(json_file))
    # For each component, contains a map from each cell name to its corresponding component
    # component name --> { cell name --> component name}
    cells_to_components = {}
    main_component = ""
    for curr_component_entry in cell_json:
        cell_map = {} # mapping cell names to component names for all cells in the current component
        if curr_component_entry["is_main_component"]:
            main_component = curr_component_entry["component"]
        for cell_info in curr_component_entry["cell_info"]:
            cell_map[cell_info["cell_name"]] = cell_info["component_name"]
        cells_to_components[curr_component_entry["component"]] = cell_map
    full_main_component = f"TOP.toplevel.{main_component}"
    components_to_cells = {main_component : [full_main_component]} # come up with a better name for this
    build_components_to_cells(full_main_component, main_component, cells_to_components, components_to_cells)
    # FIXME: extreme hack. Find a better way to do this...
    full_cell_names_to_components = {}
    for component in components_to_cells:
        for cell in components_to_cells[component]:
            full_cell_names_to_components[cell] = component

    return full_main_component, full_cell_names_to_components

def create_traces(active_element_probes_info, call_stack_probes_info, cell_caller_probes_info, total_cycles, cells_to_components, main_component):
    timeline_map = {i : set() for i in range(total_cycles)}
    # first iterate through all of the profiled info
    for unit_name in active_element_probes_info:
        unit = active_element_probes_info[unit_name]
        for segment in unit.closed_segments:
            for i in range(segment["start"], segment["end"]):
                timeline_map[i].add(unit) # maybe too memory intensive?

    new_timeline_map = {i : [] for i in range(total_cycles)}
    # now, we need to figure out the sets of traces
    for i in timeline_map:
        parents = set()
        i_mapping = {} # each unique group inv mapping to its stack. the "group" should be the last item on each stack
        i_mapping[main_component] = ["main"] # [main_component]

        cell_worklist = [main_component] # FIXME: maybe remove the hardcoding?
        while len(cell_worklist) > 0:
            current_cell = cell_worklist.pop()
            current_component = cells_to_components[current_cell]
            covered_units_in_component = set() # collect all of the units we've covered.
            # this is so silly... but catch all active units that are groups in this component.
            units_to_cover = set(filter(lambda unit: not unit.is_cell and unit.component == current_component, timeline_map[i]))
            # find all enables from control. these are all units that either (1) don't have any maps in call_stack_probes_info, or (2) have no active parent calls in call_stack_probes_info
            for active_unit in units_to_cover:
                if active_unit.is_cell: # skip cells for now as we're considering only single component programs
                    continue
                if active_unit.id() not in call_stack_probes_info: # no maps in call_stack_probes_info
                    i_mapping[active_unit.name] = i_mapping[current_cell] + [active_unit.shortname]
                    parents.add(current_cell)
                    covered_units_in_component.add(active_unit.name)
                else:
                    # loop through all parents and see if any of them are active
                    contains_active_parent = False
                    for parent, call_probe_info in call_stack_probes_info[active_unit.id()].items():
                        if call_probe_info.is_active_at_cycle(i):
                            contains_active_parent = True
                            break
                    if not contains_active_parent:
                        i_mapping[active_unit.name] = i_mapping[current_cell] + [active_unit.shortname]
                        parents.add(current_cell)
                        covered_units_in_component.add(active_unit.name)
            while len(covered_units_in_component) < len(units_to_cover):
                # loop through all other elements to figure out parent child info
                for active_unit in units_to_cover:
                    if active_unit.is_cell or active_unit.name in i_mapping:
                        continue
                    for parent, call_probe_info in call_stack_probes_info[active_unit.id()].items():
                        if f"{main_component}.{parent}" in i_mapping: # we can directly build on top of the parent
                            i_mapping[active_unit.name] = i_mapping[f"{current_cell}.{parent}"] + [active_unit.shortname]
                            covered_units_in_component.add(active_unit.name)
                        parents.add(f"{current_cell}.{parent}")
            # by this point, we should have covered all groups in the same component...
            # now we need to construct stacks for any cells that are called from a group in the current component.
            # collect caller ids in cell_caller_probes_info that belong to our component
            cell_invoker_ids = list(filter(lambda x : x.split(DELIMITER)[1] == current_component, cell_caller_probes_info))
            for cell_invoker_id in cell_invoker_ids:
                cell_invoker = cell_invoker_id.split(DELIMITER)[0]
                # iterate through all of the cells that the group invokes
                for invoked_cell_name in cell_caller_probes_info[cell_invoker_id]:
                    cell_calling_probe = cell_caller_probes_info[cell_invoker_id][invoked_cell_name]
                    cell_active_probe = active_element_probes_info[invoked_cell_name]
                    if cell_calling_probe.is_active_at_cycle(i) and cell_active_probe.is_active_at_cycle(i):
                        cell_worklist.append(cell_active_probe.name)
                        # invoker group is the parent of the cell.
                        cell_component = cells_to_components[cell_active_probe.name]
                        i_mapping[cell_active_probe.name] = i_mapping[f"{current_cell}.{cell_invoker}"] + [f"{cell_active_probe.shortname} [{cell_component}]"]
                        parents.add(f"{current_cell}.{cell_invoker}")

        for elem in i_mapping:
            if elem not in parents:
                new_timeline_map[i].append(i_mapping[elem])
        
    for i in new_timeline_map:
        print(i)
        for stack in new_timeline_map[i]:
            print(f"\t{stack}")

    return new_timeline_map

def create_tree(timeline_map):
    # ugliest implementation of a tree
    node_id_acc = 0
    tree_dict = {} # node id --> node name
    path_dict = {} # stack list string --> list of node ids
    stack_list = []
    for sl in timeline_map.values():
        for s in sl:
            if s not in stack_list:
                stack_list.append(s)
    stack_list.sort(key=len)
    for stack in stack_list:
        stack_string = ";".join(stack)
        if stack_string not in path_dict:
            id_path_list = []
            prefix = ""
            # check if we have any prefixes. start from the longest 
            for other_stack_string in sorted(path_dict, key=len, reverse=True):
                if other_stack_string in stack_string:
                    # prefix found!
                    prefix = other_stack_string
                    id_path_list = list(path_dict[other_stack_string])
                    break
            # create nodes
            if prefix != "":
                new_nodes = stack_string.split(f"{prefix};")[1].split(";")
            else:
                new_nodes = stack
            for elem in new_nodes:
                tree_dict[node_id_acc] = elem
                id_path_list.append(node_id_acc)
                node_id_acc += 1
            path_dict[stack_string] = id_path_list

    print(tree_dict)
    print(path_dict)

    return tree_dict, path_dict

def create_path_dot_str_dict(path_dict):
    path_to_dot_str = {} # stack list string --> stack path representation on dot file.

    for path_id in path_dict:
        path = path_dict[path_id]
        path_acc = ""
        for node_id in path[0:-1]:
            path_acc += f'{node_id} -> '
        path_acc += f'{path[-1]}'
        path_to_dot_str[path_id] = path_acc

    return path_to_dot_str

def create_output(timeline_map, out_dir):

    tree_dict, path_dict = create_tree(timeline_map)
    path_to_dot_str = create_path_dot_str_dict(path_dict)
    all_paths_ordered = sorted(path_dict.keys())

    os.mkdir(out_dir)
    for i in timeline_map:
        used_paths = set()
        used_nodes = set()
        all_nodes = set(tree_dict.keys())
        # figure out what nodes are used and what nodes aren't used
        for stack in timeline_map[i]:
            stack_id = ";".join(stack)
            used_paths.add(stack_id)
            for node_id in path_dict[stack_id]:
                used_nodes.add(node_id)

        fpath = os.path.join(out_dir, f"cycle{i}.dot")
        # really lazy rn but I should actually use a library for this
        with open(fpath, "w") as f:
            f.write("digraph cycle" + str(i) + " {\n")
            # declare nodes.
            # used nodes should simply be declared
            for used_node in used_nodes:
                f.write(f'\t{used_node} [label={tree_dict[used_node]}];\n')
            # unused nodes should be declared with gray
            for unused_node in all_nodes.difference(used_nodes):
                f.write(f'\t{unused_node} [label={tree_dict[unused_node]},color="{INVISIBLE}",fontcolor="{INVISIBLE}"];\n')
            # write all paths.
            for path_id in all_paths_ordered:
                if ";" not in path_id or path_id in used_paths:
                    f.write(f'\t{path_to_dot_str[path_id]} ;\n')
                else:
                    f.write(f'\t{path_to_dot_str[path_id]} [color="{INVISIBLE}"];\n')
            f.write("}")

    # make flame graph folded file
    stacks = {} # stack to number of cycles
    for i in timeline_map:
        for stack_list in timeline_map[i]:
            # stack_str = ";".join(map(lambda x : x.flame_repr(), stack_list))
            stack_id = ";".join(stack_list)
            if stack_id not in stacks:
                stacks[stack_id] = 1
            else:
                stacks[stack_id] += 1
    
    with open(os.path.join(out_dir, "flame.folded"), "w") as flame_out:
        for stack in stacks:
            flame_out.write(f"{stack} {stacks[stack]}\n")

def main(vcd_filename, cells_json_file, out_dir):
    # FIXME: will support multicomponent programs later. There's maybe something wrong here.
    main_component, cells_to_components = read_component_cell_names_json(cells_json_file)
    converter = VCDConverter(main_component, cells_to_components)
    vcdvcd.VCDVCD(vcd_filename, callbacks=converter)
    converter.postprocess()

    print("Active groups info: " + str(converter.active_elements_info.keys()))
    print()
    print("Call stack info: " + str(converter.call_stack_probe_info))
    print()
    print("Cell stack info: " + str(converter.cell_invoke_caller_probe_info))
    print()

    # NOTE: for a more robust implementation, we can even skip the part where we store active
    # cycles per group.
    new_timeline_map = create_traces(converter.active_elements_info, converter.call_stack_probe_info, converter.cell_invoke_caller_probe_info, converter.clock_cycles, cells_to_components, main_component)

    create_tree(new_timeline_map)

    create_output(new_timeline_map, out_dir)


if __name__ == "__main__":
    if len(sys.argv) > 3:
        vcd_filename = sys.argv[1]
        cells_json = sys.argv[2]
        out_dir = sys.argv[3]
        main(vcd_filename, cells_json, out_dir)
    else:
        args_desc = [
            "VCD_FILE",
            "CELLS_JSON",
            "OUT_DIR"
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        print("CELLS_JSON: Run the `component_cells` tool")
        sys.exit(-1)
