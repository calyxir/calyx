import sys
import json
import vcdvcd

def remove_size_from_name(name: str) -> str:
    """ changes e.g. "state[2:0]" to "state" """
    return name.split('[')[0]

class ProfilingInfo:
    def __init__(self, name, fsm_name=None, fsm_values=None, par_parent=None):
        self.name = name
        self.fsm_name = fsm_name
        self.fsm_values = fsm_values
        self.total_cycles = 0
        self.closed_segments = [] # Segments will be (start_time, end_time)
        self.current_segment = None
        if par_parent == "":
            self.par_parent = None
        else:
            self.par_parent = par_parent

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

def can_start_new_segment(vcd_converter, group, signal_new_value, signal_prev_value, time, curr_clock_cycle):
    par_parent = vcd_converter.profiling_info[group].par_parent
    is_new_signal = signal_new_value != signal_prev_value # Did the value change between the previous cycle?

    if par_parent is not None:
        # if group == "read":
        #     print(f"READ detected! curr cycle: {curr_clock_cycle}")
        #     print(f"par_parent is active? {vcd_converter.profiling_info[par_parent].is_active()}")
        #     print(f"par_parent start time {vcd_converter.profiling_info[par_parent].start_clock_cycle()}")
        if not vcd_converter.profiling_info[par_parent].is_active(): # No child segments should start before the parent starts
            return False
        if vcd_converter.profiling_info[par_parent].start_clock_cycle() == curr_clock_cycle: # All child segments start when the parent starts
            return True

    if vcd_converter.main_go_on_time == time: # All active segments start when main starts
        return True
    
    return is_new_signal
    

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
            self.profiling_info[group] = ProfilingInfo(group, groups_to_fsms[group]["fsm"], groups_to_fsms[group]["ids"], groups_to_fsms[group]["par_parent"])
        for (single_enable_group, single_enable_parent) in single_enable_names:
            self.profiling_info[single_enable_group] = ProfilingInfo(single_enable_group, par_parent=single_enable_parent)
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
            # fixpoint - we might analyze the par parent of a group before the group.
            # TODO: this is getting really messy........
            started_group_names = set()
            while True:
                prev_started_group_names_len = len(started_group_names)
                # for each signal that we want to check, we need to sample the values
                for (signal_name, signal_id) in self.signal_to_signal_id.items():
                    signal_new_value = int(cur_sig_vals[signal_id], 2) # signal value at this point in time
                    fsm_curr_value = self.signal_to_curr_value[signal_name]
                    if "_go" in signal_name and signal_new_value == 1:
                        # start of single enable group
                        group = "_".join(signal_name.split("_")[0:-1])
                        curr_group_info = self.profiling_info[group]
                        # We want to start a segment regardless of whether it changed
                        if (group not in started_group_names and
                            can_start_new_segment(self, group, signal_new_value, fsm_curr_value, time, self.clock_cycle_acc)):
                            curr_group_info.start_new_segment(self.clock_cycle_acc)
                            started_group_names.add(curr_group_info)
                    elif "_done" in signal_name and signal_new_value == 1:
                        # end of single enable group
                        group = "_".join(signal_name.split("_")[0:-1])
                        self.profiling_info[group].end_current_segment(self.clock_cycle_acc)
                    elif "fsm" in signal_name:
                        next_group = self.fsms[signal_name][signal_new_value]
                        # start a new segment for the next group
                        if (next_group not in started_group_names and
                            can_start_new_segment(self, next_group, signal_new_value, fsm_curr_value, time, self.clock_cycle_acc)):
                            if fsm_curr_value != -1:
                                # end the previous group if there was one
                                prev_group = self.fsms[signal_name][fsm_curr_value]
                                self.profiling_info[prev_group].end_current_segment(self.clock_cycle_acc)
                            self.profiling_info[next_group].start_new_segment(self.clock_cycle_acc)
                            started_group_names.add(next_group)
                    # Update internal signal value
                    self.signal_to_curr_value[signal_name] = signal_new_value                
                if len(started_group_names) == prev_started_group_names_len: # Fixpoint: if none of the groups started in this iteration of check
                    break


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
                group_name = state["group"]
                if group_name not in groups_to_fsms:
                    groups_to_fsms[group_name] = {"fsm": fsm_name, "ids": [state["id"]], "par_parent" : fsm["parent_group"]}
                else:     
                    groups_to_fsms[group_name]["ids"].append(state["id"])  
        else:
            group_info = (profiling_info["SingleEnable"]["group"], profiling_info["SingleEnable"]["parent_group"])
            single_enable_names.add(group_info)

    return fsms, single_enable_names, groups_to_fsms


def main(vcd_filename, json_file):
    fsms, single_enable_names, groups_to_fsms = remap_tdcc_json(json_file)
    converter = VCDConverter(fsms, single_enable_names, groups_to_fsms)
    vcdvcd.VCDVCD(vcd_filename, callbacks=converter, store_tvs=False)
    print(f"Total clock cycles: {converter.clock_cycle_acc}")
    print("=====SUMMARY=====")
    print()
    for group_info in converter.profiling_info.values():
        print(group_info.summary())
    print("=====DUMP=====")
    print()
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
