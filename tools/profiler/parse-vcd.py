import sys
import json
import vcdvcd

def remove_size_from_name(name: str) -> str:
    """ changes e.g. "state[2:0]" to "state" """
    return name.split('[')[0]

class ProfilingInfo:
    def __init__(self, name, fsm_name=None, fsm_value=None):
        self.name = name
        self.fsm_name = fsm_name
        self.fsm_value = fsm_value
        self.total_cycles = 0
        self.segments = [] # Segments will be (start_time, end_time)

    def __repr__ (self):
        # Remove any non-closed segments
        segment_repr = []
        for segment in self.segments:
            if segment["end"] != -1:
                segment_repr.append(segment)
        return str({"group-name" : self.name, "fsm-name": self.fsm_name, "group-fsm-value": self.fsm_value, "total-cycles": self.total_cycles, "segments": segment_repr})

    def start_new_segment(self, curr_clock_cycle):
        self.segments.append({"start": curr_clock_cycle, "end": -1})

    def end_current_segment(self, curr_clock_cycle):
        if len(self.segments) > 0:
            # Close out previous segment by setting the end time to the current cycle            
            if (self.segments[-1]["end"] == -1): # ignore cases where done is high forever
                self.segments[-1]["end"] = curr_clock_cycle
                self.total_cycles += curr_clock_cycle - self.segments[-1]["start"]

class VCDConverter(vcdvcd.StreamParserCallbacks):

    def __init__(self, fsms, single_enable_names, groups_to_fsms):
        super().__init__()
        self.fsms = fsms
        self.single_enable_names = single_enable_names
        self.profiling_info = {}
        self.signal_to_signal_id = {fsm : None for fsm in fsms}
        self.signal_to_curr_value = {fsm : 0 for fsm in fsms}
        self.main_go_id = None
        self.main_go_on = False
        self.main_go_on_time = None
        self.clock_id = None
        self.clock_cycle_acc = -1 # The 0th clock cycle will be 0.
        for group in groups_to_fsms:
            fsm_name, fsm_value = groups_to_fsms[group]
            self.profiling_info[group] = ProfilingInfo(group, fsm_name, fsm_value)
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
            # We may want to optimize these nested for loops
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
            # for each signal that we want to check, we need to sample the values
            for (signal_name, signal_id) in self.signal_to_signal_id.items():
                signal_new_value = int(cur_sig_vals[signal_id], 2) # signal value at this point in time
                fsm_curr_value = self.signal_to_curr_value[signal_name]
                # skip values that have not changed, except for when main[go] just got activated
                if not(self.main_go_on_time == time) and signal_new_value == fsm_curr_value:
                    continue
                if "_go" in signal_name and signal_new_value == 1:
                    # start of single enable group
                    group = "_".join(signal_name.split("_")[0:-1])
                    self.profiling_info[group].start_new_segment(self.clock_cycle_acc)
                elif "_done" in signal_name and signal_new_value == 1:
                    # end of single enable group
                    group = "_".join(signal_name.split("_")[0:-1])
                    self.profiling_info[group].end_current_segment(self.clock_cycle_acc)
                elif "fsm" in signal_name:
                    # Sample FSM values
                    if fsm_curr_value != -1:
                        # end the previous group if there was one
                        prev_group = self.fsms[signal_name][fsm_curr_value]
                        self.profiling_info[prev_group].end_current_segment(self.clock_cycle_acc)
                    if signal_new_value in self.fsms[signal_name]: # END should be ignored
                        next_group = self.fsms[signal_name][signal_new_value]
                        # start a new segment for the next group
                        # FIXME: need to fix this for par blocks
                        self.profiling_info[next_group].start_new_segment(self.clock_cycle_acc)
                    else:
                        # The state id was not in the JSON entry for this FSM. Most likely the value was the last FSM state.
                        print(f"FSM value ignored: {signal_new_value}")
                # Update internal signal value
                self.signal_to_curr_value[signal_name] = signal_new_value

def remap_tdcc_json(json_file):
    profiling_infos = json.load(open(json_file))
    single_enable_names = set()
    groups_to_fsms = {}
    fsms = {} # Remapping of JSON data for easy access
    for profiling_info in profiling_infos:
        if "Fsm" in profiling_info:
            fsm = profiling_info["Fsm"]
            fsm_name = fsm["fsm"]
            fsms[fsm_name] = {}
            for state in fsm["states"]:
                fsms[fsm_name][state["id"]] = state["group"]
                groups_to_fsms[state["group"]] = (fsm_name, state["id"])
        else:
            group_name = profiling_info["SingleEnable"]["group"]
            single_enable_names.add(group_name)

    return fsms, single_enable_names, groups_to_fsms


def main(vcd_filename, json_file):
    fsms, single_enable_names, groups_to_fsms = remap_tdcc_json(json_file)
    converter = VCDConverter(fsms, single_enable_names, groups_to_fsms)
    vcdvcd.VCDVCD(vcd_filename, callbacks=converter, store_tvs=False)
    print(f"Total clock cycles: {converter.clock_cycle_acc}")
    for group_info in converter.profiling_info.values():
        print(group_info)

if __name__ == "__main__":
    if len(sys.argv) > 2:
        vcd_filename = sys.argv[1]
        fsm_json = sys.argv[2]
        main(vcd_filename, fsm_json)
    else:
        args_desc = [
            "VCD_FILE",
            "TDCC_JSON"
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        sys.exit(-1)
