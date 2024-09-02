import csv
import sys
import json
import vcdvcd

def remove_size_from_name(name: str) -> str:
    """ changes e.g. "state[2:0]" to "state" """
    return name.split('[')[0]

class ProfilingInfo:
    def __init__(self, name, fsm_name=None, fsm_values=None, tdcc_group_name=None):
        self.name = name
        self.fsm_name = fsm_name
        self.fsm_values = fsm_values
        self.total_cycles = 0
        self.closed_segments = [] # Segments will be (start_time, end_time)
        self.current_segment = None
        self.tdcc_group = tdcc_group_name

    def __repr__ (self):
        segments_str = ""
        for segment in self.closed_segments:
            if (segments_str != ""):
                segments_str += ", "
            segments_str += f"[{segment['start']}, {segment['end']})"
        if self.fsm_name is None:
            header = f"[GT]  Group {self.name}:\n"
        else:
            header = (f"[FSM] Group {self.name}:\n" + 
            f"\tFSM name: {self.fsm_name}\n" +
            f"\tFSM state ids: {self.fsm_values}\n"
            )

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
        return {"name": self.name, 
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

    def __init__(self, fsms, single_enable_names, tdcc_group_names, fsm_group_maps, cells_to_components, main_component):
        super().__init__()
        self.main_component = main_component
        self.fsms = fsms
        self.single_enable_names = single_enable_names
        self.cells_to_components = cells_to_components
        # Recording the first cycle when the TDCC group became active
        self.tdcc_group_active_cycle = {tdcc_group_name : -1 for tdcc_group_name in tdcc_group_names}
        self.tdcc_group_to_go_id = {tdcc_group_name : None for tdcc_group_name in tdcc_group_names}
        self.profiling_info = {}
        self.signal_to_signal_id = {fsm : None for fsm in fsms}
        self.signal_to_curr_value = {fsm : -1 for fsm in fsms}
        self.main_go_id = None
        self.main_go_on = False
        self.main_go_on_time = None
        self.clock_id = None
        self.clock_cycles = -1 # the 0th clock cycle will be 0
        for group in fsm_group_maps:
            self.profiling_info[f"{group}FSM"] = ProfilingInfo(group, fsm_group_maps[group]["fsm"], fsm_group_maps[group]["ids"], fsm_group_maps[group]["tdcc-group-name"])
        for single_enable_group in single_enable_names:
            self.profiling_info[single_enable_group] = ProfilingInfo(single_enable_group)
            self.signal_to_curr_value[f"{single_enable_group}_go"] = -1
            self.signal_to_curr_value[f"{single_enable_group}_done"] = -1
        # probably need to clean up a lot of stuff later, but adding new stuff first
        self.timestamps_to_events = {}

    def enddefinitions(self, vcd, signals, cur_sig_vals):
        # convert references to list and sort by name
        refs = [(k, v) for k, v in vcd.references_to_ids.items()]
        refs = sorted(refs, key=lambda e: e[0])
        names = [remove_size_from_name(e[0]) for e in refs]
        signal_id_dict = {sid : [] for sid in vcd.references_to_ids.values()} # one id can map to multiple signal names since wires are connected
        main_go_name = f"{self.main_component}.go"
        self.main_go_id = vcd.references_to_ids[main_go_name]
        signal_id_dict[self.main_go_id] = [main_go_name]

        clock_name = f"{self.main_component}.clk"
        if clock_name not in names:
            print("Can't find the clock? Exiting...")
            sys.exit(1)
        self.clock_id = vcd.references_to_ids[clock_name]
        signal_id_dict[self.clock_id] = [clock_name]

        for name, sid in refs:
            # We may want to optimize these nested for loops
            for tdcc_group in self.tdcc_group_to_go_id:
                if name.startswith(f"{tdcc_group}_go.out["):
                    self.tdcc_group_to_go_id[tdcc_group] = sid
                    signal_id_dict[sid].append(name)
            for fsm in self.fsms:
                if name.startswith(f"{fsm}.out["):
                    self.signal_to_signal_id[fsm] = sid
                    signal_id_dict[sid].append(name)
            for single_enable_group in self.single_enable_names:
                if name.startswith(f"{single_enable_group}_go.out["):
                    self.signal_to_signal_id[f"{single_enable_group}_go"] = sid
                    signal_id_dict[sid].append(name)
                if name.startswith(f"{single_enable_group}_done.out["):
                    self.signal_to_signal_id[f"{single_enable_group}_done"] = sid
                    signal_id_dict[sid].append(name)

        # don't need to check for signal ids that don't pertain to signals we're interested in
        self.signal_id_to_names = {k:v for k,v in signal_id_dict.items() if len(v) > 0}

    def value(
        self,
        vcd,
        time,
        value,
        identifier_code,
        cur_sig_vals,
    ):
        # Start profiling after main's go is on
        if identifier_code == self.main_go_id and value == "1":
            self.main_go_on_time = time
            # FIXME: probably want to do some setup here (some fsms starting at 0 n stuff)
        if self.main_go_on_time is None:
            return
        
        # ignore all signals we don't care about
        if identifier_code not in self.signal_id_to_names:
            return
        
        if time not in self.timestamps_to_events:
            self.timestamps_to_events[time] = []

        signal_names = self.signal_id_to_names[identifier_code]
        int_value = int(value, 2)

        for signal_name in signal_names:
            self.timestamps_to_events[time].append({"signal": signal_name, "value": int_value})
        # # detect rising edge on clock
        # if identifier_code == self.clock_id and value == "1":
        #     self.clock_cycle_acc += 1

        # for signal_name in signal_names:
        #     print(f"[{time}] Signal name: {signal_name}, value: {value}")
        #     if "tdcc" in signal_name:
        #         continue
        #     if "_go" in signal_name and int_value == 1:
        #         print("GO!!!")
        #         # start of group ground truth
        #         group = "_".join(signal_name.split("_")[0:-1])
        #         curr_group_info = self.profiling_info[group]
        #         print(signal_name)
        #         print(curr_group_info)
        #         # We want to start a segment regardless of whether it changed
        #         # if self.main_go_on_time == time or signal_new_value != signal_curr_value:
        #         curr_group_info.start_new_segment(self.clock_cycle_acc)
        #     elif "_done" in signal_name and int_value == 1:
        #         # end of group ground truth
        #         group = "_".join(signal_name.split("_")[0:-1])
        #         self.profiling_info[group].end_current_segment(self.clock_cycle_acc)

            # # Update TDCC group signals first
            # for (tdcc_group_name, tdcc_signal_id) in self.tdcc_group_to_go_id.items():
            #     tdcc_group_is_active = int(cur_sig_vals[tdcc_signal_id], 2) == 1
            #     if self.tdcc_group_active_cycle[tdcc_group_name] == -1 and tdcc_group_is_active: # the tdcc group just became active
            #         self.tdcc_group_active_cycle[tdcc_group_name] = self.clock_cycle_acc
            #     elif self.tdcc_group_active_cycle[tdcc_group_name] > -1 and not tdcc_group_is_active:
            #         self.tdcc_group_active_cycle[tdcc_group_name] = -1
            # # for each signal that we want to check, we need to sample the values
            # for (signal_name, signal_id) in self.signal_to_signal_id.items():
            #     signal_curr_value = self.signal_to_curr_value[signal_name]
            #     signal_new_value = int(cur_sig_vals[signal_id], 2) # signal value at this point in time
                

                # elif "fsm" in signal_name:
                #     # Workarounds because the value 0 may not correspond to a group
                #     if signal_curr_value in self.fsms[signal_name]:
                #         # group that is recorded to be active last cycle. If the signal changed then it would be the previous group
                #         curr_group = self.fsms[signal_name][signal_curr_value]
                #         # if the FSM value changed, then we must end the current group (regardless of whether we can start the next group)
                #         if signal_new_value != signal_curr_value and signal_curr_value != -1:
                #             self.profiling_info[curr_group].end_current_segment(self.clock_cycle_acc)
                #     if signal_new_value in self.fsms[signal_name]:
                #         next_group = self.fsms[signal_name][signal_new_value]
                #         tdcc_group_active_cycle = self.tdcc_group_active_cycle[self.profiling_info[next_group].tdcc_group]
                #         if tdcc_group_active_cycle == -1: # If the TDCC group is not active, then no segments should start
                #             continue
                #         # if the FSM value didn't change but the TDCC group just got enabled, then we must start the next group
                #         if signal_new_value == signal_curr_value and tdcc_group_active_cycle == self.clock_cycle_acc:
                #             self.profiling_info[next_group].start_new_segment(self.clock_cycle_acc)
                #         elif signal_new_value != signal_curr_value: # otherwise we start a new segment when the signal changed
                #             self.profiling_info[next_group].start_new_segment(self.clock_cycle_acc)
                # # Update internal signal value
                # self.signal_to_curr_value[signal_name] = signal_new_value                

    # Postprocess data mapping timestamps to events (signal changes)
    # We have to postprocess instead of processing signals in a stream because
    # signal changes that happen at the same time as a clock tick might be recorded
    # *before* or *after* the clock change on the VCD file (hence why we can't process
    # everything within a stream if we wanted to be precise)
    def postprocess(self):
        clock_name = f"{self.main_component}.clk"
        clock_cycles = -1
        fsm_to_active_group = {fsm : None for fsm in self.fsms}
        print(fsm_to_active_group)
        for ts in self.timestamps_to_events:
            events = self.timestamps_to_events[ts]
            # checking whether the timestamp has a rising edge (hacky)
            if {"signal": clock_name, "value": 1} in events:
                clock_cycles += 1
            # TDCC groups need to be recorded for tracking FSM values
            # (ex. if the FSM has value 0 but the TDCC group isn't active, then the group represented by the 
            # FSM's 0 value should not be considered as active)
            for tdcc_event in filter(lambda e : "tdcc_go" in e["signal"], events):
                tdcc_group = "_".join(tdcc_event["signal"].split("_")[0:-1])
                if self.tdcc_group_active_cycle[tdcc_group] == -1 and tdcc_event["value"] == 1: # value changed to 1
                    self.tdcc_group_active_cycle[tdcc_group] = clock_cycles
                    # FIXME: we need to start all of the FSMs that depend on this TDCC group here
                elif self.tdcc_group_active_cycle[tdcc_group] > -1 and tdcc_event["value"] == 0: # tdcc group that was active's signal turned to 0
                    self.tdcc_group_active_cycle[tdcc_group] = -1
            for event in events:
                signal_name = event["signal"]
                value = event["value"]
                if "tdcc_go" in signal_name: # skip all tdcc events since we've already processed them
                    continue
                if "_go" in signal_name and value == 1:
                    group = "_".join(signal_name.split("_")[0:-1])
                    self.profiling_info[group].start_new_segment(clock_cycles)
                elif "_done" in signal_name and value == 1:
                    group = "_".join(signal_name.split("_")[0:-1])
                    self.profiling_info[group].end_current_segment(clock_cycles)
                elif "fsm" in signal_name:
                    fsm = ".".join(signal_name.split(".")[0:-1])
                    # need the original
                    # Workarounds because the value 0 may not correspond to a group
                    if fsm_to_active_group[fsm] is not None:
                        prev_group = f"{fsm_to_active_group[fsm]}FSM" # getting the "FSM" variant of the group
                        self.profiling_info[prev_group].end_current_segment(clock_cycles)
                    if value in self.fsms[fsm]:
                        next_group = f"{self.fsms[fsm][value]}FSM"  # getting the "FSM" variant of the group
                        print(self.profiling_info[next_group].tdcc_group)
                        tdcc_group_active_cycle = self.tdcc_group_active_cycle[self.profiling_info[next_group].tdcc_group]
                        if tdcc_group_active_cycle == -1: # If the TDCC group is not active, then no segments should start
                            print("continue")
                            continue
                        self.profiling_info[next_group].start_new_segment(clock_cycles)


        self.clock_cycles = clock_cycles

# Generates a list of all of the components to potential cell names
# prefix is the cell's "path" (ex. for a cell "my_cell" defined in "main", the prefix would be "TOP.toplevel.main")
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
    group_names = set() # all groups (to record ground truth)
    tdcc_group_names = set() # TDCC-generated groups that manage control flow using FSMs
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
                        fsm_group_maps[group_name] = {"fsm": fsm_name, "tdcc-group-name": tdcc_group, "ids": [state["id"]]}
                        tdcc_group_names.add(tdcc_group) # Keep track of the TDCC group to figure out when first group starts
                        group_names.add(group_name)
                    else:     
                        fsm_group_maps[group_name]["ids"].append(state["id"])  
        else:
            for cell in components_to_cells[profiling_info["SingleEnable"]["component"]]: # get all possibilities of cells
                group_names.add(cell + "." + profiling_info["SingleEnable"]["group"])

    return fsms, group_names, tdcc_group_names, fsm_group_maps

def output_result(out_csv, dump_out_json, converter):
    print("="*50)
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
    fsms, group_names, tdcc_group_names, fsm_group_maps = remap_tdcc_json(groups_json_file, components_to_cells)
    print(f"GROUP NAMES: {group_names}")
    converter = VCDConverter(fsms, group_names, tdcc_group_names, fsm_group_maps, components_to_cells, main_component)
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
