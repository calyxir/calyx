import sys
import json
from vcdvcd import VCDVCD

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
    vcd = VCDVCD(vcd_filename)
    fsm_val_to_num_cycles = {}
    for key in vcd.references_to_ids.keys():
        if f"{component_name}.fsm_out" in key:
            signal = vcd[key]
            fsm_value = -1
            fsm_time_start = -1
            for signal_value_tuple in signal.tv:
                curr_time, binary_fsm_val = signal_value_tuple
                if fsm_value >= 0:
                    # FIXME: For now, will assume that each clock cycle takes 10 ms
                   fsm_val_to_num_cycles[fsm_values[fsm_value]] = int((curr_time - fsm_time_start) / 10)
                fsm_value = int(binary_fsm_val, 2)
                fsm_time_start = curr_time

    print(fsm_val_to_num_cycles)


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

