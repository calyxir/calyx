import csv
import sys
import json
import vcdvcd

def remove_size_from_name(name: str) -> str:
    """ changes e.g. "state[2:0]" to "state" """
    return name.split('[')[0]

class ProfilingInfo:
    def __init__(self, name, component, fsm_name=None, fsm_values=None, tdcc_group_name=None, is_cell=False):
        self.name = name
        self.fsm_name = fsm_name
        self.fsm_values = fsm_values
        self.total_cycles = 0
        self.closed_segments = [] # Segments will be (start_time, end_time)
        self.current_segment = None
        self.tdcc_group = tdcc_group_name
        self.component = component
        self.is_cell = is_cell

    def __repr__ (self):
        segments_str = ""
        for segment in self.closed_segments:
            if (segments_str != ""):
                segments_str += ", "
            segments_str += f"[{segment['start']}, {segment['end']})"
        if self.fsm_name is not None:
            header = (f"[FSM] Group {self.name}:\n" + 
            f"\tFSM name: {self.fsm_name}\n" +
            f"\tFSM state ids: {self.fsm_values}\n"
            )
        elif self.component is None:
            header = f"[CMP] Group {self.name}:\n"
        else:
            header = f"[GT]  Group {self.name}:\n"

        return (header +
        f"\tTotal cycles: {self.total_cycles}\n" +
        f"\t# of times active: {len(self.closed_segments)}\n" +
        f"\tSegments: {segments_str}\n"
        )

    def is_active(self):
        return self.current_segment is not None

    def start_clock_cycle(self):
        if self.current_segment is None:
            return -1
        else:
            return self.current_segment["start"]

    def compute_average_cycles(self):
        if len(self.closed_segments) == 0:
            return 0
        else:
            return round(self.total_cycles / len(self.closed_segments), 2)

    def emit_csv_data(self):
        name = self.name
        if self.fsm_name is not None:
            name += "[FSM]"
        if self.component is None:
            name += "[CMP]"
        return {"name": name, 
                "total-cycles" : self.total_cycles,
                "times-active" : len(self.closed_segments),
                "avg" : self.compute_average_cycles()}

    def summary(self):
        if self.fsm_name is None:
            header = "[GT] "
        else:
            header = "[FSM]"
        return (f"{header} Group {self.name} Summary:\n" +
        f"\tTotal cycles: {self.total_cycles}\n" +
        f"\t# of times active: {len(self.closed_segments)}\n" +
        f"\tAvg runtime: {self.compute_average_cycles()}\n"
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

    def __init__(self, fsms, single_enable_names, tdcc_groups, fsm_group_maps, main_component, cells):
        super().__init__()
        self.main_component = main_component
        self.fsms = fsms
        self.single_enable_names = single_enable_names.keys()
        # Recording the first cycle when the TDCC group became active
        self.tdcc_group_active_cycle = {tdcc_group_name : -1 for tdcc_group_name in tdcc_groups}
        # Map from a TDCC group to all FSMs that depend on it. maybe a 1:1 mapping
        self.tdcc_group_to_dep_fsms = tdcc_groups
        # Group name --> ProfilingInfo object
        self.profiling_info = {}
        self.signal_to_curr_value = {fsm : -1 for fsm in fsms}
        for group in fsm_group_maps:
            # Differentiate FSM versions from ground truth versions
            self.profiling_info[f"{group}FSM"] = ProfilingInfo(group, fsm_group_maps[group]["component"], fsm_group_maps[group]["fsm"], fsm_group_maps[group]["ids"], fsm_group_maps[group]["tdcc-group-name"])
        for single_enable_group in single_enable_names:
            self.profiling_info[single_enable_group] = ProfilingInfo(single_enable_group, single_enable_names[single_enable_group])
            self.signal_to_curr_value[f"{single_enable_group}_go"] = -1
            self.signal_to_curr_value[f"{single_enable_group}_done"] = -1
        self.cells = set(cells.keys())
        for cell in cells:
            self.profiling_info[cell] = ProfilingInfo(cell, cells[cell], is_cell=True)
        # Map from timestamps [ns] to value change events that happened on that timestamp
        self.timestamps_to_events = {}

    def enddefinitions(self, vcd, signals, cur_sig_vals):
        # convert references to list and sort by name
        refs = [(k, v) for k, v in vcd.references_to_ids.items()]
        refs = sorted(refs, key=lambda e: e[0])
        names = [remove_size_from_name(e[0]) for e in refs]
        signal_id_dict = {sid : [] for sid in vcd.references_to_ids.values()} # one id can map to multiple signal names since wires are connected

        # main_go_name = f"{self.main_component}.go"
        # signal_id_dict[vcd.references_to_ids[main_go_name]] = [main_go_name]

        clock_name = f"{self.main_component}.clk"
        if clock_name not in names:
            print("Can't find the clock? Exiting...")
            sys.exit(1)
        signal_id_dict[vcd.references_to_ids[clock_name]] = [clock_name]

        # get go and done for cells (the signals are exactly {cell}.go and {cell}.done)
        for cell in self.cells:
            cell_go = cell + ".go"
            cell_done = cell + ".done"
            if cell_go not in vcd.references_to_ids:
                print(f"Not accounting for cell {cell} (probably combinational)")
                continue
            signal_id_dict[vcd.references_to_ids[cell_go]].append(cell_go)
            signal_id_dict[vcd.references_to_ids[cell_done]].append(cell_done)

        for name, sid in refs:
            # FIXME: We may want to optimize these nested for loops
            for tdcc_group in self.tdcc_group_to_dep_fsms:
                if name.startswith(f"{tdcc_group}_go.out["):
                    signal_id_dict[sid].append(name)
            for fsm in self.fsms:
                if name.startswith(f"{fsm}.out["):
                    signal_id_dict[sid].append(name)
            for single_enable_group in self.single_enable_names:
                if name.startswith(f"{single_enable_group}_go.out["):
                    signal_id_dict[sid].append(name)
                if name.startswith(f"{single_enable_group}_done.out["):
                    signal_id_dict[sid].append(name)

        # don't need to check for signal ids that don't pertain to signals we're interested in
        self.signal_id_to_names = {k:v for k,v in signal_id_dict.items() if len(v) > 0}

    # Stream processes the events recorded in the VCD and stores them in self.timestamps_to_events
    # NOTE: Stream processing doesn't work because value changes that happen in the same timestamp
    # are not processed at the same time.
    # NOTE: when we reimplement this script, we probably want to separate this part from the
    # clock cycle processing
    def value(
        self,
        vcd,
        time,
        value,
        identifier_code,
        cur_sig_vals,
    ):
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
        fsm_to_active_group = {fsm : None for fsm in self.fsms}
        # current values of FSM registers. This is different from fsm_to_active_group since the TDCC group for the FSM
        # may not be active (which means that no group managed by the FSM is active)
        fsm_to_curr_value = {fsm: -1 for fsm in self.fsms}
        started = False
        for ts in self.timestamps_to_events:
            events = self.timestamps_to_events[ts]
            started = started or [x for x in events if x["signal"] == f"{self.main_component}.go" and x["value"] == 1]
            if not started:
                # Update fsm_to_curr_value for any FSM signals that got updated. We will start the events corresponding
                # to those values once the TDCC group for the FSM starts.
                # Realistically this will most likely only happen on the 0th cycle just to set the FSM value to 0,
                # but trying to be extra safe here.
                for event in filter(lambda e : "fsm" in e["signal"], events):
                    fsm = ".".join(event["signal"].split(".")[0:-1])
                    if event["value"] in self.fsms[fsm]:
                        fsm_to_curr_value[fsm] = event["value"]
                continue
            # checking whether the timestamp has a rising edge (hacky)
            if {"signal": clock_name, "value": 1} in events:
                clock_cycles += 1
            # TDCC groups need to be recorded (before FSMs) for tracking FSM values
            # (ex. if the FSM has value 0 but the TDCC group isn't active, then the group represented by the
            # FSM's 0 value should not be considered as active)
            for tdcc_event in filter(lambda e : "tdcc" in e["signal"] and "go" in e["signal"], events):
                tdcc_group = "_".join(tdcc_event["signal"].split("_")[0:-1])
                if self.tdcc_group_active_cycle[tdcc_group] == -1 and tdcc_event["value"] == 1: # value changed to 1
                    self.tdcc_group_active_cycle[tdcc_group] = clock_cycles
                    for fsm in self.tdcc_group_to_dep_fsms[tdcc_group]:
                        value = fsm_to_curr_value[fsm]
                        if value != -1:
                            if value not in self.fsms[fsm]:
                                continue
                            next_group = f"{self.fsms[fsm][value]}FSM"
                            fsm_to_active_group[fsm] = next_group
                            self.profiling_info[next_group].start_new_segment(clock_cycles)
                elif self.tdcc_group_active_cycle[tdcc_group] > -1 and tdcc_event["value"] == 0: # tdcc group that was active's signal turned to 0
                    self.tdcc_group_active_cycle[tdcc_group] = -1
            for event in events:
                signal_name = event["signal"]
                value = event["value"]
                if "tdcc" in signal_name and "go" in signal_name: # skip all tdcc events since we've already processed them
                    continue
                if signal_name.endswith(".go") and value == 1: # cells have .go and .done
                    cell = signal_name.split(".go")[0]
                    self.profiling_info[cell].start_new_segment(clock_cycles)
                if signal_name.endswith(".done") and value == 1: # cells have .go and .done
                    cell = signal_name.split(".done")[0]
                    self.profiling_info[cell].end_current_segment(clock_cycles)
                if "_go" in signal_name and value == 1:
                    group = "_".join(signal_name.split("_")[0:-1])
                    self.profiling_info[group].start_new_segment(clock_cycles)
                elif "_done" in signal_name and value == 1:
                    group = "_".join(signal_name.split("_")[0:-1])
                    self.profiling_info[group].end_current_segment(clock_cycles)
                elif "fsm" in signal_name:
                    fsm = ".".join(signal_name.split(".")[0:-1])
                    fsm_to_curr_value[fsm] = value
                    # Workarounds because the value 0 may not correspond to a group
                    if fsm_to_active_group[fsm] is not None:
                        prev_group = fsm_to_active_group[fsm] # getting the "FSM" variant of the group
                        self.profiling_info[prev_group].end_current_segment(clock_cycles)
                    if value in self.fsms[fsm]:
                        next_group = f"{self.fsms[fsm][value]}FSM"  # getting the "FSM" variant of the group
                        tdcc_group_active_cycle = self.tdcc_group_active_cycle[self.profiling_info[next_group].tdcc_group]
                        if tdcc_group_active_cycle == -1: # If the TDCC group is not active, then no segments should start
                            continue
                        fsm_to_active_group[fsm] = next_group
                        self.profiling_info[next_group].start_new_segment(clock_cycles)

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
    return full_main_component, components_to_cells

# Reads json generated by TDCC (via dump-fsm-json option) to produce initial group information
def remap_tdcc_json(json_file, components_to_cells):
    profiling_infos = json.load(open(json_file))
    group_names = {} # all groups (to record ground truth). Maps to the group's component (needed for stacks)
    cells_to_components = {} # go and done info are needed for cells. cell --> component name
    tdcc_groups = {} # TDCC-generated groups that manage control flow using FSMs. maps to all fsms that map to the tdcc group
    fsm_group_maps = {} # fsm-managed groups info (fsm register, TDCC group that manages fsm, id of group within fsm)
    fsms = {} # Remapping of JSON data for easy access
    for profiling_info in profiling_infos:
        if "Fsm" in profiling_info:
            fsm = profiling_info["Fsm"]
            # create entries for all possible cells of component
            for cell in components_to_cells[fsm["component"]]:
                fsm_name = cell + "." + fsm["fsm"]
                fsms[fsm_name] = {}
                for state in fsm["states"]:
                    group_name = cell + "." + state["group"]
                    fsms[fsm_name][state["id"]] = group_name
                    tdcc_group = cell + "." + fsm["group"]
                    if group_name not in fsm_group_maps:
                        fsm_group_maps[group_name] = {"fsm": fsm_name, "tdcc-group-name": tdcc_group, "ids": [state["id"]], "component": fsm["component"]}
                        if tdcc_group not in tdcc_groups: # Keep track of the TDCC group to figure out when first group starts
                            tdcc_groups[tdcc_group] = set()
                        tdcc_groups[tdcc_group].add(fsm_name)
                        group_names[group_name] = fsm["component"]
                    else:
                        fsm_group_maps[group_name]["ids"].append(state["id"])  
        else:
            component = profiling_info["SingleEnable"]["component"]
            for cell in components_to_cells[component]: # get all possibilities of cells
                group_names[cell + "." + profiling_info["SingleEnable"]["group"]] = component
    for component in components_to_cells:
        for cell in components_to_cells[component]:
            cells_to_components[cell] = component

    return fsms, group_names, tdcc_groups, fsm_group_maps, cells_to_components

def output_result(out_csv, dump_out_json, converter):
    print(f"Total clock cycles: {converter.clock_cycles}")
    print("=====SUMMARY=====")
    print()
    groups_to_emit = list(filter(lambda group : not group.name.startswith("tdcc") and not group.name.endswith("END"), converter.profiling_info.values()))
    groups_to_emit.sort(key=lambda x : x.name) # to preserve stability
    groups_to_emit.sort(key=lambda x : x.total_cycles, reverse=True)
    csv_acc = []
    dump_json_acc = []
    for group_info in groups_to_emit:
        csv_acc.append(group_info.emit_csv_data())
        dump_json_acc.append(group_info.__dict__)
        print(group_info.summary())
    print("=====DUMP=====")
    print()
    for group_info in groups_to_emit:
        print(group_info)
    # Add total cycles for visualizer script (probably want to do this in a neater fashion in the future)
    dump_json_acc.append({"name": "TOTAL", "total_cycles": converter.clock_cycles, "main_full_path": converter.main_component})
    # emit a json for visualizer script
    print(f"Writing dump JSON to {dump_out_json}")
    with open(dump_out_json, "w", encoding="utf-8") as dump_file:
        dump_file.write(json.dumps(dump_json_acc, indent=4))
    # emitting a CSV file for easier eyeballing
    print(f"Writing summary to {out_csv}")
    csv_keys = ["name", "total-cycles", "times-active", "avg"]
    csv_acc.append({ "name": "TOTAL", "total-cycles": converter.clock_cycles, "times-active": "-", "avg": "-"})
    if (out_csv == "STDOUT"):
        writer = csv.DictWriter(sys.stdout, csv_keys, lineterminator="\n")
    else:
        writer = csv.DictWriter(open(out_csv, "w"), csv_keys, lineterminator="\n")
    writer.writeheader()
    writer.writerows(csv_acc)

def main(vcd_filename, groups_json_file, cells_json_file, out_csv, dump_out_json):
    main_component, components_to_cells = read_component_cell_names_json(cells_json_file)
    fsms, group_names, tdcc_group_names, fsm_group_maps, cells = remap_tdcc_json(groups_json_file, components_to_cells)
    converter = VCDConverter(fsms, group_names, tdcc_group_names, fsm_group_maps, main_component, cells)
    vcdvcd.VCDVCD(vcd_filename, callbacks=converter, store_tvs=False)
    converter.postprocess()
    output_result(out_csv, dump_out_json, converter)

if __name__ == "__main__":
    if len(sys.argv) > 5:
        vcd_filename = sys.argv[1]
        fsm_json = sys.argv[2]
        cells_json = sys.argv[3]
        out_csv = sys.argv[4]
        dump_out_json = sys.argv[5]
        main(vcd_filename, fsm_json, cells_json, out_csv, dump_out_json)
    else:
        args_desc = [
            "VCD_FILE",
            "TDCC_JSON",
            "CELLS_JSON",
            "SUMMARY_OUT_CSV",
            "DUMP_OUT_JSON"
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        print("TDCC_JSON: Run Calyx with `tdcc:dump-fsm-json` option")
        print("CELLS_JSON: Run Calyx with `component-cells` backend")
        print("If SUMMARY_OUT_CSV is STDOUT, then summary CSV will be printed to stdout")
        sys.exit(-1)
