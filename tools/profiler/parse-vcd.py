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
        return (f"Group {self.name}:\n" +
        f"\tFSM name: {self.fsm_name}\n" +
        f"\tFSM state ids: {self.fsm_values}\n" +
        f"\tTotal cycles: {self.total_cycles}\n" +
        f"\tSegments: {self.closed_segments}\n"
        )

    def is_active(self):
        return self.current_segment is not None

    def start_clock_cycle(self):
        if self.current_segment is None:
            return -1
        else:
            return self.current_segment["start"]

    def summary(self):
        if len(self.closed_segments) == 0:
            average_cycles = 0
        else:
            average_cycles = self.total_cycles / len(self.closed_segments)
        return (f"Group {self.name} Summary:\n" +
        f"\tTotal cycles: {self.total_cycles}\n" +
        f"\t# of times active: {len(self.closed_segments)}\n" +
        f"\tAvg runtime: {average_cycles}\n"
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

    def __init__(self, fsms, single_enable_names, tdcc_group_names, groups_to_fsms, cells_to_components):
        super().__init__()
        self.fsms = fsms
        self.single_enable_names = single_enable_names
        self.cells_to_components = cells_to_components
        self.tdcc_group_to_values = {tdcc_group_name : [] for tdcc_group_name in tdcc_group_names}
        self.tdcc_group_to_go_id = {tdcc_group_name : None for tdcc_group_name in tdcc_group_names}
        self.profiling_info = {}
        self.signal_to_signal_id = {fsm : None for fsm in fsms}
        self.signal_to_curr_value = {fsm : 0 for fsm in fsms}
        self.main_go_id = None
        self.main_go_on = False
        self.main_go_on_time = None
        self.clock_id = None
        self.clock_cycle_acc = -1 # The 0th clock cycle will be 0.
        for group in groups_to_fsms:
            self.profiling_info[group] = ProfilingInfo(group, groups_to_fsms[group]["fsm"], groups_to_fsms[group]["ids"], groups_to_fsms[group]["tdcc-group-name"])
        for single_enable_group in single_enable_names:
            self.profiling_info[single_enable_group] = ProfilingInfo(single_enable_group)
            self.signal_to_curr_value[f"{single_enable_group}_go"] = -1
            self.signal_to_curr_value[f"{single_enable_group}_done"] = -1

    def enddefinitions(self, vcd, signals, cur_sig_vals):
        # convert references to list and sort by name
        refs = [(k, v) for k, v in vcd.references_to_ids.items()]
        refs = sorted(refs, key=lambda e: e[0])
        names = [remove_size_from_name(e[0]) for e in refs]

        # FIXME: When we get to profiling multi-component programs, we want to search for each component's go signal
        self.main_go_id = vcd.references_to_ids["TOP.TOP.main.go"]

        clock_name = "TOP.TOP.main.clk"
        if clock_name in names:
            self.clock_id = vcd.references_to_ids[clock_name]
        else:
            print("Can't find the clock? Exiting...")
            sys.exit(1)

        for name, id in refs:
            # print(name, id)
            # We may want to optimize these nested for loops
            for tdcc_group in self.tdcc_group_to_go_id:
                if f"{tdcc_group}_go.out[" in name:
                    self.tdcc_group_to_go_id[tdcc_group] = id
            for fsm in self.fsms:
                if f"{fsm}.out[" in name:
                    self.signal_to_signal_id[fsm] = id
            for single_enable_group in self.single_enable_names:
                if f"{single_enable_group}_go.out[" in name:
                    self.signal_to_signal_id[f"{single_enable_group}_go"] = id
                if f"{single_enable_group}_done.out[" in name:
                    self.signal_to_signal_id[f"{single_enable_group}_done"] = id

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
        if self.main_go_on_time is None :
            return

        # detect rising edge on clock
        if identifier_code == self.clock_id and value == "1":
            self.clock_cycle_acc += 1
            # Update TDCC group signals first
            for (tdcc_group_name, tdcc_signal_id) in self.tdcc_group_to_go_id.items():
                self.tdcc_group_to_values[tdcc_group_name].append(int(cur_sig_vals[tdcc_signal_id], 2))
            # for each signal that we want to check, we need to sample the values
            for (signal_name, signal_id) in self.signal_to_signal_id.items():
                signal_new_value = int(cur_sig_vals[signal_id], 2) # signal value at this point in time
                fsm_curr_value = self.signal_to_curr_value[signal_name]
                if "_go" in signal_name and signal_new_value == 1:
                    # start of single enable group
                    group = "_".join(signal_name.split("_")[0:-1])
                    curr_group_info = self.profiling_info[group]
                    # We want to start a segment regardless of whether it changed
                    if self.main_go_on_time == time or signal_new_value != fsm_curr_value:
                        curr_group_info.start_new_segment(self.clock_cycle_acc)
                elif "_done" in signal_name and signal_new_value == 1:
                    # end of single enable group
                    group = "_".join(signal_name.split("_")[0:-1])
                    self.profiling_info[group].end_current_segment(self.clock_cycle_acc)
                elif "fsm" in signal_name:
                    next_group = self.fsms[signal_name][signal_new_value]
                    tdcc_group_values = self.tdcc_group_to_values[self.profiling_info[next_group].tdcc_group]
                    # if the FSM value changed, then we must end the previous group (regardless of whether we can start the next group)
                    if signal_new_value != fsm_curr_value and fsm_curr_value != -1:
                        prev_group = self.fsms[signal_name][fsm_curr_value]
                        self.profiling_info[prev_group].end_current_segment(self.clock_cycle_acc)
                    # if the FSM value didn't change but the TDCC group just got enabled, then we must start the next group
                    if signal_new_value == fsm_curr_value and (tdcc_group_values[-1] == 1 and (len(tdcc_group_values) == 1 or tdcc_group_values[-2] == 0)):
                        self.profiling_info[next_group].start_new_segment(self.clock_cycle_acc)
                    if tdcc_group_values[-1] == 1 and signal_new_value != fsm_curr_value:
                        self.profiling_info[next_group].start_new_segment(self.clock_cycle_acc)
                # Update internal signal value
                self.signal_to_curr_value[signal_name] = signal_new_value                

# Not sure if I need this, but generating a list of all of the components to potential cell names
def make_mappings(prefix, curr_component, cells_to_components, mapping):
    prefix = prefix + "." + curr_component
    for (cell, cell_component) in cells_to_components[curr_component].items():
        mapping[cell_component] = prefix + "." + cell
        make_mappings(prefix, cell_component, cells_to_components, mapping)


def read_component_cell_names_json(json_file): # TODO: the keys in the json may change
    component_cell_infos = json.load(open(json_file))
    # For each component, contains a map from each cell name to its corresponding component
    # component name --> { cell name --> component name}
    cells_to_components = {} 
    for item in component_cell_infos:
        cell_map = {} # mapping cell names to component names for all cells in the current component
        for cell_info in item["cell_info"]:
            cell_map[cell_info["name"]] = cell_info["component"]
        cells_to_components[item["component"]] = cell_map
    # FIXME: assuming for now that "TOP.TOP.main" is the toplevel component. Need to fix this
    mapping = {"main" : "TOP.TOP.main"} # come up with a better name for this
    make_mappings("TOP.TOP", "main", cells_to_components, mapping)
    print(mapping)
    return cells_to_components

def remap_tdcc_json(json_file):
    profiling_infos = json.load(open(json_file))
    single_enable_names = set()
    tdcc_group_names = set()
    groups_to_fsms = {}
    fsms = {} # Remapping of JSON data for easy access
    for profiling_info in profiling_infos:
        if "Fsm" in profiling_info:
            fsm = profiling_info["Fsm"]
            fsm_name = fsm["fsm"]
            fsms[fsm_name] = {}
            for state in fsm["states"]:
                fsms[fsm_name][state["id"]] = state["group"]
                group_name = state["group"]
                if group_name not in groups_to_fsms:
                    groups_to_fsms[group_name] = {"fsm": fsm_name, "tdcc-group-name": fsm["group"], "ids": [state["id"]]}
                    tdcc_group_names.add(fsm["group"]) # Hack: Keep track of the TDCC group for use later
                else:     
                    groups_to_fsms[group_name]["ids"].append(state["id"])  
        else:
            single_enable_names.add(profiling_info["SingleEnable"]["group"])

    return fsms, single_enable_names, tdcc_group_names, groups_to_fsms


def main(vcd_filename, groups_json_file, cells_json_file):
    cells_to_components = read_component_cell_names_json(cells_json_file)
    fsms, single_enable_names, tdcc_group_names, groups_to_fsms = remap_tdcc_json(groups_json_file)
    converter = VCDConverter(fsms, single_enable_names, tdcc_group_names, groups_to_fsms, cells_to_components)
    vcdvcd.VCDVCD(vcd_filename, callbacks=converter, store_tvs=False)
    print(f"Total clock cycles: {converter.clock_cycle_acc}")
    print("=====SUMMARY=====")
    print()
    for group_info in filter(lambda group : not group.name.startswith("tdcc") and not group.name.endswith("END"), converter.profiling_info.values()):
        print(group_info.summary())
    print("=====DUMP=====")
    print()
    for group_info in filter(lambda group : not group.name.startswith("tdcc") and not group.name.endswith("END"), converter.profiling_info.values()):
        print(group_info)

if __name__ == "__main__":
    if len(sys.argv) > 3:
        vcd_filename = sys.argv[1]
        fsm_json = sys.argv[2]
        cells_json = sys.argv[3]
        main(vcd_filename, fsm_json, cells_json)
    else:
        args_desc = [
            "VCD_FILE",
            "TDCC_JSON",
            "CELLS_JSON"
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        sys.exit(-1)
