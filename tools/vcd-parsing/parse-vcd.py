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


# def read_values():

def main(vcd_filename, json_file):
    component_name, fsm_values = remap_tdcc_json(json_file)
    vcd = VCDVCD(vcd_filename)
    fsm_val_to_num_cycles = {}

    # need to ignore until the main component starts
    main_go_signals = vcd["TOP.TOP.main.go"]
    start_time_ms = 0
    for go_time, go_val in main_go_signals.tv:
        print(f"go-time: {go_time}, go_val: {go_val}")
        if int(go_val, 2) == 1:
            start_time_ms = go_time

    print(f"Main starttime: {start_time_ms}")


    # FIXME? for now we assume that FSM values do not change before the main component starts (seems reasonable)
    # fsm_to_tv_idx = {key : 0 for key in vcd.references_to_ids.keys() if f"{component_name}.fsm_out" in key }
    # # Detect starting index
    # for key in vcd.references_to_ids.keys():
    #     if f"{component_name}.fsm_out" in key:
    #         start_idx = 0
    #         for time, val in vcd[key].tv:
    #             if 

    # FIXME: make things really simple for myself by hardcoding the FSM name
    fsm_tv = vcd["TOP.TOP.main.fsm_out"].tv
    fsm_tv_idx = 0
    fsm_value = -1

    clock_signals = vcd["TOP.TOP.main.clk"] # hardcoding top level clock
    for clk_idx in range(len(clock_signals.tv)):
        clock_time, clock_signal = 
        if clock_time < start_time_ms: # ignore values before main actually starts
            continue
        if int(clock_signal, 2) == 0: # FIXME? Sampling at falling edge for now
            next_fsm_time, next_fsm_bin_val = fsm_tv[fsm_tv_idx+1]
            if (next_fsm_time < )



    # for key in vcd.references_to_ids.keys():
    #     signal = vcd[key]
    #     fsm_value = -1
    #     fsm_time_start = -1
    #     for signal_value_tuple in signal.tv:
    #         curr_time, binary_fsm_val = signal_value_tuple
    #         if fsm_value >= 0:
    #             # FIXME: For now, will assume that each clock cycle takes 10 ms
    #            fsm_val_to_num_cycles[fsm_values[fsm_value]] = int((curr_time - fsm_time_start) / 10)
    #         fsm_value = int(binary_fsm_val, 2)
    #         fsm_time_start = curr_time

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

