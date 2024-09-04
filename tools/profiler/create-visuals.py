# Takes in a dump file created by parse-vcd.py and creates a JSON file in the Google Trace Event Format
import json
import sys

class FlameInfo:
    def __init__(self, name, backptr, cycles):
        self.name = name
        self.backptr = backptr
        self.cycles = cycles

    def make_folded_log_entry(self):
        if self.backptr is not None:
            return f'{self.backptr};{self.name} {self.cycles}'
        else:
            return f'{self.name} {self.cycles}'

# Creates folded log
def create_flame_graph(profiled_info, flame_out, fsm_flame_out):
    summary = list(filter(lambda x : x["name"] == "TOTAL", profiled_info))[0]
    main_component = summary["main_full_path"]
    total_cycles = summary["total_cycles"]
    # FIXME: maybe make an object instead of a dictionary
    stacks = [FlameInfo(main_component, None, total_cycles)]
    # FIXME: NOT going to deal with multicompoinent programs for now
    # in the future I think we need to create and chase back pointers
    for group_info in profiled_info:
        if group_info["name"] == "TOTAL": # already processed the summary
            continue
        # FIXME: ignore the FSM entries for now. Later we probably want to make a separate folded log for those
        if group_info["fsm_name"] is None:
            name = group_info["name"].split(".")[-1] # FIXME: assumes that there are no multicomponent programs... need to do better here
            backptr = main_component # FIXME: will NOT be true for multicomponent
            cycles = group_info["total_cycles"]
            stacks.append(FlameInfo(name, backptr, cycles))
    with open(flame_out, "w") as f:
        for entry in stacks:
            f.write(entry.make_folded_log_entry() + "\n")

# Starting with the JSON array format for now... [Needs to be fixed]
# example
# [ {"name": "Asub", "cat": "PERF", "ph": "B", "pid": 22630, "tid": 22630, "ts": 829},
#   {"name": "Asub", "cat": "PERF", "ph": "E", "pid": 22630, "tid": 22630, "ts": 833} ]
def create_timeline_view(profiled_info, out_file):
    cat = "GT" # Ground truth category (will overwrite if it's FSM)
    events = []
    id_acc = 1
    ts_multiplier = 100 # some arbitrary number to multiply by so that it's easier to see in the viewer
    for group_info in profiled_info:
        if group_info["name"] == "TOTAL": # timeline view doesn't need a total time
            continue
        name = group_info["name"].split("TOP.toplevel.", 1)[1]
        if group_info["fsm_name"] is not None:
            cat = "FSM"
            name = "[FSM] " + name
        for segment in group_info["closed_segments"]:
            # beginning of segment
            begin_time = segment["start"] * ts_multiplier
            events.append({"name": name, "cat": cat, "ph": "B", "pid" : id_acc, "tid": id_acc, "ts" : begin_time})
            # end of segment
            end_time = segment["end"] * ts_multiplier
            events.append({"name": name, "cat": cat, "ph": "E", "pid": id_acc, "tid": id_acc, "ts": end_time})
        id_acc += 1
    with open(out_file, "w") as out:
        json.dump(events, out, indent=4)

def main(profiler_dump_file, timeline_out, flame_out, fsm_flame_out):
    profiled_info = json.load(open(profiler_dump_file, "r"))
    create_timeline_view(profiled_info, timeline_out)
    create_flame_graph(profiled_info, flame_out, fsm_flame_out)

if __name__ == "__main__":
    if len(sys.argv) > 4:
        profiler_dump_json = sys.argv[1]
        timeline_out = sys.argv[2]
        flame_out = sys.argv[3]
        fsm_flame_out = sys.argv[4]
        main(profiler_dump_json, timeline_out, flame_out, fsm_flame_out)
    else:
        args_desc = [
            "PROFILER_JSON",
            "TIMELINE_VIEW_JSON",
            "FLAME_GRAPH_FOLDED",
            "FSM_FLAME_GRAPH_FOLDED",
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        sys.exit(-1)
