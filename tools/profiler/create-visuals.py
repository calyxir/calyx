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

# def create_backptrs(name, main_component, cells_map):
#     after_main = prefix.split(f"{main_component}.")[1]

#     return

def create_timeline_map(profiled_info):
    summary = list(filter(lambda x : x["name"] == "TOTAL", profiled_info))[0]
    total_cycles = summary["total_cycles"]
    timeline_map = {i : set() for i in range(total_cycles)}
    timeline_map_fsm = {i : set() for i in range(total_cycles)}
    for group_info in profiled_info:
        if group_info["name"] == "TOTAL" or group_info["component"] is None: # only care about actual groups
            continue
        if group_info["fsm_name"] is not None: # FIXME: get FSM version of events later as well
            continue
        for segment in group_info["closed_segments"]:
            for i in range(segment["start"], segment["end"]): # really janky, I wonder if there's a better way to do this?
                timeline_map[i].add((group_info["name"], group_info["component"]))

    print(timeline_map)
    return timeline_map

# attempt to rehash the create_flame_graph to take care of stacks
def create_flame_graph_2(profiled_info, flame_out):
    timeline = create_timeline_map(profiled_info)
    summary = list(filter(lambda x : x["name"] == "TOTAL", profiled_info))[0]
    main_component = summary["main_full_path"]
    main_shortname = main_component.split("TOP.toplevel.")[1]
    total_cycles = summary["total_cycles"]
    stacks = {} # each stack to the # of cycles it was active for?
    for i in timeline: # keys in the timeline are clock time stamps
        for group_full_name, group_component in timeline[i]:
            stack = ""
            group_name = group_full_name.split(".")[-1]
            if group_component == main_shortname: # group within main so it has to be at the bottom
                stack = main_component + ";" + group_name
            else:
                stack = ";".join((main_component, group_component, group_name)) # FIXME: temp
            
            if stack not in stacks:
                stacks[stack] = 0
            stacks[stack] += 1

    print("STACKS")
    print(stacks)
    return stacks

# Creates folded log
def create_flame_graph(profiled_info, cells_map, flame_out, fsm_flame_out):
    summary = list(filter(lambda x : x["name"] == "TOTAL", profiled_info))[0]
    main_component = summary["main_full_path"]
    total_cycles = summary["total_cycles"]
    stacks = {}
    for group_info in profiled_info:
        if group_info["name"] == "TOTAL" or group_info["name"] == main_component: # already processed the summary
            continue
        name_split = group_info["name"].split(".")
        name = name_split[-1] # FIXME: still not correct for multicomponent programs
        prefix = ".".join(name_split[:-1])
        if prefix == main_component:
            backptr = prefix # base case?
        else: # multicomponent
            after_main = prefix.split(f"{main_component}.")[1]
            backptr = main_component
            # for cell in after_main.split("."):
                
            # after_main.replace(".", ";")
        cycles = group_info["total_cycles"]
        if name not in stacks:
            stacks[name] = {}
        if group_info["fsm_name"] is None:
            stacks[name]["gt"] = FlameInfo(name, backptr, cycles, False)
        else:
            stacks[name]["fsm"] = FlameInfo(name, backptr, cycles, True)
    f = open(flame_out, "w")
    f_fsm = open(fsm_flame_out, "w")
    # The cycle count entry for main_component needs to be the *difference* between all groups and the total number
    # of cycles
    gt_aggregate = 0
    fsm_aggregate = 0
    for group_name in stacks:
        entry = stacks[group_name]
        gt_aggregate += entry["gt"].cycles
        f.write(entry["gt"].make_folded_log_entry() + "\n")
        if "fsm" in entry:
            fsm_aggregate += entry["fsm"].cycles
            f_fsm.write(entry["fsm"].make_folded_log_entry() + "\n")
        else:
            fsm_aggregate += entry["gt"].cycles
            f_fsm.write(entry["gt"].make_folded_log_entry() + "\n")
    f.write(FlameInfo(main_component, None, max(total_cycles - gt_aggregate, 0), False).make_folded_log_entry() + "\n")
    f_fsm.write(FlameInfo(main_component, None, max(total_cycles - fsm_aggregate, 0), False).make_folded_log_entry() + "\n")

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
    return {x["name"] : x for x in cell_json}        

def main(profiler_dump_file, cells_json, timeline_out, flame_out, fsm_flame_out):
    profiled_info = json.load(open(profiler_dump_file, "r"))
    create_flame_graph_2(profiled_info, flame_out)
    # create_timeline_view(profiled_info, timeline_out)
    # create_flame_graph(profiled_info, cells_json, flame_out, fsm_flame_out)

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
