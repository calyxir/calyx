# Takes in a dump file created by parse-vcd.py and creates a JSON file in the Google Trace Event Format
import json
import sys

class FlameInfo:
    def __init__(self, name, backptr, cycles, is_fsm):
        self.name = name
        self.backptr = backptr
        self.cycles = cycles
        self.is_fsm = is_fsm

    def make_folded_log_entry(self):
        if self.backptr is not None:
            return f'{self.backptr};{self.name} {self.cycles}'
        else:
            return f'{self.name} {self.cycles}'

def create_timeline_map(profiled_info):
    summary = list(filter(lambda x : x["name"] == "TOTAL", profiled_info))[0]
    total_cycles = summary["total_cycles"]
    timeline_map = {i : {} for i in range(total_cycles)}
    for group_info in profiled_info:
        if group_info["name"] == "TOTAL" or group_info["component"] is None: # only care about actual groups
            continue
        if group_info["fsm_name"] is not None: # FIXME: get FSM version of events later as well
            continue
        for segment in group_info["closed_segments"]:
            for i in range(segment["start"], segment["end"]): # really janky, I wonder if there's a better way to do this?
                timeline_map[i][group_info["component"]] = group_info["name"]

    return timeline_map

# attempt to rehash the create_flame_graph to take care of stacks
def create_flame_graph(profiled_info, cells_map, flame_out):
    timeline = create_timeline_map(profiled_info)
    summary = list(filter(lambda x : x["name"] == "TOTAL", profiled_info))[0]
    main_component = summary["main_full_path"]
    main_shortname = main_component.split("TOP.toplevel.")[1]
    # total_cycles = summary["total_cycles"]
    stacks = {} # each stack to the # of cycles it was active for?
    nonactive_cycles = 0 # cycles where no group was active
    for i in timeline: # keys in the timeline are clock time stamps
        # Right now we are assuming that there are no pars. So for any time stamp, *if there are multiple* groups active,
        # we need to find the one that is the longest (since that's the innermost one).
        # NOTE: This might be generalizable for even the 1 group active case? Going to try it out
        if len(timeline[i]) == 0:
            nonactive_cycles += 1
            continue
        group_component = sorted(timeline[i], key=lambda k : timeline[i][k].count("."), reverse=True)[0]
        group_full_name = timeline[i][group_component]
        stack = ""
        group_name = group_full_name.split(".")[-1]
        if group_component == main_shortname:
            stack = main_component + ";" + group_name
        else:
            after_main = group_full_name.split(f"{main_component}.")[1]
            after_main_split = after_main.split(".")[:-1]
            # first, find the group in main that is simulatenous
            if main_shortname not in timeline[i]:
                print(f"Error: A group from the main component ({main_shortname}) should be active at cycle {i}!")
                exit(1)
            backptrs = [main_component]
            group_from_main = timeline[i][main_shortname].split(main_component + ".")[-1]
            backptrs.append(group_from_main)
            prev_component = main_shortname
            for cell_name in after_main_split:
                cell_component = cells_map[prev_component][cell_name]
                group_from_component = timeline[i][cell_component].split(cell_name + ".")[-1]
                backptrs.append(f"{cell_component}[{prev_component}.{cell_name}];{group_from_component}")
                prev_component = cell_component
            stack = ";".join(backptrs)
            
        if stack not in stacks:
            stacks[stack] = 0
        stacks[stack] += 1

    stacks[main_component] = nonactive_cycles

    with open(flame_out, "w") as f:
        for stack in sorted(stacks, key=lambda k : len(k)): # main needs to come first for flame graph script to not make two boxes for main?
            f.write(f"{stack}  {stacks[stack]}\n")

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

def build_cells_map(json_file):
    cell_json = json.load(open(json_file))
    cells_map = {}
    for component_entry in cell_json:
        inner_cells_map = {}
        for cell_entry in component_entry["cell_info"]:
            inner_cells_map[cell_entry["cell_name"]] = cell_entry["component_name"]
        cells_map[component_entry["component"]] = inner_cells_map
    return cells_map

def main(profiler_dump_file, cells_json, timeline_out, flame_out, fsm_flame_out):
    profiled_info = json.load(open(profiler_dump_file, "r"))
    # This cells_map is different from the one in parse-vcd.py
    cells_map = build_cells_map(cells_json)
    create_flame_graph(profiled_info, cells_map, flame_out)
    # create_timeline_view(profiled_info, timeline_out)

if __name__ == "__main__":
    if len(sys.argv) > 5:
        profiler_dump_json = sys.argv[1]
        cells_json = sys.argv[2]
        timeline_out = sys.argv[3]
        flame_out = sys.argv[4]
        fsm_flame_out = sys.argv[5]
        main(profiler_dump_json, cells_json, timeline_out, flame_out, fsm_flame_out)
    else:
        args_desc = [
            "PROFILER_JSON",
            "CELLS_JSON",
            "TIMELINE_VIEW_JSON",
            "FLAME_GRAPH_FOLDED",
            "FSM_FLAME_GRAPH_FOLDED",
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        sys.exit(-1)
