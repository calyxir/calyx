import sys
import json
import vcdvcd

def remove_size_from_name(name: str) -> str:
    """ changes e.g. "state[2:0]" to "state" """
    return name.split('[')[0]

class VCDConverter(vcdvcd.StreamParserCallbacks):

    # def __init__(self, out: typing.TextIO, interesting_signals: list, max_depth: int = -1):

    def __init__(self, fsm_values):
        super().__init__()
        self.fsm_values = fsm_values
        self.fsm_val_to_num_cycles = {group_name: 0 for group_name in fsm_values.values()}
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
            print(f"Clock ID: {self.clock_id}")
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
        # print('{} {} {}'.format(time, value, identifier_code))
        
        # When a value changes

        # First need to check if main component is going
        if identifier_code == self.main_go_id and value == "1":
            self.main_go_on = True
            print("[AYAKA] Main is on!!!!")
        if not(self.main_go_on):
            return

        # detect falling edge on clock
        if identifier_code == self.clock_id and value == "0":
            self.clock_cycle_acc += 1
            print(f"Current # clock cycles: {self.clock_cycle_acc}")
            # Sample FSM values
            fsm_curr_value = cur_sig_vals[self.fsm_signal_id]
            self.fsm_curr_value = int(fsm_curr_value, 2)
            print(f"Current value of FSM: {self.fsm_curr_value}")
            if self.fsm_curr_value in self.fsm_values:
                self.fsm_val_to_num_cycles[self.fsm_values[self.fsm_curr_value]] += 1
        

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
    print(converter.fsm_val_to_num_cycles)


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
