import csv
import json
import os
import re
import shutil
import sys
import vcdvcd

def remove_size_from_name(name: str) -> str:
    """ changes e.g. "state[2:0]" to "state" """
    return name.split('[')[0]

class StackTree:
    def __init__(self, name, data):
        self.key = name
        self.data = data
        self.children = []


class ProfilingInfo:
    def __init__(self, probe_encoded_name, is_cell=False):
        if is_cell:
            self.name = probe_encoded_name
            self.callsite = None
            self.component = None
        else:
            encoding_split = probe_encoded_name.split("__")
            self.name = encoding_split[0]
            self.callsite = encoding_split[1]
            self.component = encoding_split[2]
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
            self.current_segment = {"start": curr_clock_cycle, "end": -1}
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

class VCDConverter(vcdvcd.StreamParserCallbacks):
    def __init__(self, main_component, cells_to_components):
        # NOTE: assuming single-component programs for now
        super().__init__()
        self.main_component = main_component
        self.probes = set()
        # Map from timestamps [ns] to value change events that happened on that timestamp
        self.timestamps_to_events = {}
        self.cells = cells_to_components
        self.profiling_info = {}
        for cell in self.cells:
            self.profiling_info[cell] = ProfilingInfo(cell, is_cell=True)

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
            if "_probe_out" in name: # this signal is a probe.
                # FIXME: this doesn't work when control invokes a group twice
                probe_encoded_name = name.split("_probe_out")[0]
                self.profiling_info[probe_encoded_name] = ProfilingInfo(probe_encoded_name)
                # group_component_split = name.split("_probe_out")[0].split("__")
                # group_name = group_component_split[0]
                # self.single_enable_names.add(group_name)
                # self.profiling_info[group_name] = ProfilingInfo(group_name, group_component_split[1])
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
                    self.profiling_info[cell].start_new_segment(clock_cycles)
                    currently_active.add(cell)
                if signal_name.endswith(".done") and value == 1: # cells have .go and .done
                    cell = signal_name.split(".done")[0]
                    self.profiling_info[cell].end_current_segment(clock_cycles)
                    currently_active.remove(cell)
                if "_probe_out" in signal_name and value == 1: # instrumented group started being active
                    encoded_info = signal_name.split("_probe_out")[0]
                    self.profiling_info[encoded_info].start_new_segment(clock_cycles)
                    currently_active.add(encoded_info)
                elif "_probe_out" in signal_name and value == 0: # instrumented group stopped being active
                    encoded_info = signal_name.split("_probe_out")[0]
                    self.profiling_info[encoded_info].end_current_segment(clock_cycles)
                    currently_active.remove(encoded_info)
        for active in currently_active: # anything that is still around...
            self.profiling_info[active].end_current_segment(clock_cycles)

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

def create_traces(profiled_info, total_cycles, main_component):
    timeline_map = {i : set() for i in range(total_cycles)}
    # first iterate through all of the profiled info
    for unit_name in profiled_info:
        unit = profiled_info[unit_name]
        for segment in unit.closed_segments:
            for i in range(segment["start"], segment["end"]):
                timeline_map[i].add(unit) # maybe too memory intensive?

    new_timeline_map = {i : [] for i in range(total_cycles)}
    # now, we need to figure out the sets of traces
    for i in timeline_map:
        parents = set()
        i_mapping = {} # each unique group inv mapping to its stack. the "group" should be the last item on each stack
        # FIXME: probably want to wrap this around a while loop or sth?
        curr_component = main_component
        main_component_info = list(filter(lambda x : x.name == main_component, timeline_map[i]))[0]
        i_mapping[main_component_info] = [main_component_info]
        # find all of the invocations from control
        for elem in filter((lambda x: x.callsite is not None and "instrumentation_wrapper" in x.callsite), timeline_map[i]):
            i_mapping[elem] = i_mapping[main_component_info] + [elem]
            parents.add(main_component_info)
        # now, walk through everything else before saturation
        new_groups = set()
        started = False
        while not started or len(new_groups) == 1:
            started = True
            new_groups = set()
            for elem in timeline_map[i]:
                if elem in i_mapping:
                    continue
                parent_find_attempt = list(filter(lambda x : x.shortname == elem.callsite, i_mapping))
                if len(parent_find_attempt) == 1: # found a match!
                    parent_info = parent_find_attempt[0]
                    i_mapping[elem] = i_mapping[parent_info] + [elem]
                    parents.add(parent_info)
                    new_groups.add(elem)

        for elem in i_mapping:
            if elem not in parents:
                new_timeline_map[i].append(i_mapping[elem])
        
    for i in new_timeline_map:
        print(i)
        for stack in new_timeline_map[i]:
            print(f"\t{stack}")

    return new_timeline_map

def create_output(timeline_map, out_dir):
    shutil.rmtree(out_dir)
    os.mkdir(out_dir)
    for i in timeline_map:
        fpath = os.path.join(out_dir, f"cycle{i}.dot")
        # really lazy rn but I should actually use a library for this
        with open(fpath, "w") as f:
            f.write("digraph cycle" + str(i) + " {\n")
            for stack in timeline_map[i]:
                acc = "\t"
                for stack_elem in stack[0:-1]:
                    acc += stack_elem.shortname + " -> "
                acc += stack[-1].shortname + ";\n"
                f.write(acc)
            f.write("}")

    # make flame graph folded file
    stacks = {} # stack to number of cycles
    for i in timeline_map:
        for stack_list in timeline_map[i]:
            stack_str = ";".join(map(lambda x : x.flame_repr(), stack_list))
            if stack_str not in stacks:
                stacks[stack_str] = 1
            else:
                stacks[stack_str] += 1
    
    with open(os.path.join(out_dir, "flame.folded"), "w") as flame_out:
        for stack in stacks:
            flame_out.write(f"{stack} {stacks[stack]}\n")

def main(vcd_filename, cells_json_file, out_dir):
    # FIXME: will support multicomponent programs later. There's maybe something wrong here.
    main_component, cells_to_components = read_component_cell_names_json(cells_json_file)
    converter = VCDConverter(main_component, cells_to_components)
    vcdvcd.VCDVCD(vcd_filename, callbacks=converter)
    converter.postprocess()

    # NOTE: for a more robust implementation, we can even skip the part where we store active
    # cycles per group.
    new_timeline_map = create_traces(converter.profiling_info, converter.clock_cycles, main_component)

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
