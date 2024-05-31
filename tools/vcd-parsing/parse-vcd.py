import sys
import json
import vcdvcd

def remove_size_from_name(name: str) -> str:
    """ changes e.g. "state[2:0]" to "state" """
    return name.split('[')[0]

class ProfilingInfo:
    def __init__(self, name, fsm_value):
        self.name = name
        self.fsm_value = fsm_value
        self.total_cycles = 0
        self.segments = [] # Segments will be (start_time, end_time)

    def __repr__ (self):
        return str({"group-name" : self.name, "group-fsm-value": self.fsm_value, "total-cycles": self.total_cycles, "segments": self.segments})

    def start_new_segment(self, curr_clock_cycle):
        self.segments.append({"start": curr_clock_cycle, "end": -1})

    def end_current_segment(self, curr_clock_cycle):
        if len(self.segments) > 0:
            # Close out previous segment by setting the end time to the current cycle
            self.segments[-1]["end"] = curr_clock_cycle
            self.total_cycles = curr_clock_cycle - self.segments[-1]["start"]

class VCDConverter(vcdvcd.StreamParserCallbacks):

    def __init__(self, fsm_values):
        super().__init__()
        self.fsm_values = fsm_values
        self.profiling_info = {}
        for (fsm_value, group_name) in fsm_values.items():
            self.profiling_info[fsm_value] = ProfilingInfo(group_name, fsm_value)
        self.main_go_id = None
        self.main_go_on = False
        self.clock_id = None
        self.fsm_signal_id = None
        self.fsm_curr_value = -1
        self.clock_cycle_acc = -1 # The 0th clock cycle will be 0.
        
    def enddefinitions(self, vcd, signals, cur_sig_vals):
        # convert references to list and sort by name
        refs = [(k, v) for k, v in vcd.references_to_ids.items()]
        refs = sorted(refs, key=lambda e: e[0])
        names = [remove_size_from_name(e[0]) for e in refs]

        # FIXME: is this ok to hardcode?
        self.main_go_id = vcd.references_to_ids["TOP.TOP.main.go"]

        clock_name = "TOP.TOP.main.clk"
        if clock_name in names:
            self.clock_id = vcd.references_to_ids[clock_name]
        else:
            print("Can't find the clock? Exiting...")
            sys.exit(1)

        for entry in refs:
            if "fsm.out" in entry[0]: # FIXME: remove assumption that there's only one FSM
                self.fsm_signal_id = entry[1]

    def value(
        self,
        vcd,
        time,
        value,
        identifier_code,
        cur_sig_vals,
    ):
        # First need to check if main component is going
        if identifier_code == self.main_go_id and value == "1":
            self.main_go_on = True
        if not(self.main_go_on):
            return

        # detect falling edge on clock
        if identifier_code == self.clock_id and value == "0":
            self.clock_cycle_acc += 1
            # Sample FSM values
            fsm_curr_value = int(cur_sig_vals[self.fsm_signal_id], 2)
            if fsm_curr_value != self.fsm_curr_value:
                # detect change!
                if self.fsm_curr_value != -1:
                    # end the previous group if there was one
                    self.profiling_info[self.fsm_curr_value].end_current_segment(self.clock_cycle_acc)
                if fsm_curr_value in self.profiling_info: # END should be ignored
                    # start the next group
                    # FIXME: need to fix this for parallelism
                    self.profiling_info[fsm_curr_value].start_new_segment(self.clock_cycle_acc)
                    self.fsm_curr_value = fsm_curr_value
                else:
                    print(f"New FSM value ignored: {fsm_curr_value}")

def remap_tdcc_json(json_file):
    tdcc_json = json.load(open(json_file))
    tdcc_json = tdcc_json[0] # FIXME: we assume that the program yields only one FSM.
    component_name = tdcc_json["component"]
    tdcc_remap = {}
    for state in tdcc_json["states"]:
        tdcc_remap[state["id"]] = state["group"]
    return component_name, tdcc_remap

def main(vcd_filename, json_file):
    component_name, fsm_values = remap_tdcc_json(json_file)
    converter = VCDConverter(fsm_values)
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
