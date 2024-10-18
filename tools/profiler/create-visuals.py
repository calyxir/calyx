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
        
class CallStackElement:
    # A component on the stack that is active at a given snapshot in time
    # starting_group is the first group that shows up
    def __init__(self, component, starting_group):
        self.component = component
        if starting_group is None:
            self.active_groups = []
        else:
            self.active_groups = [starting_group]
            self.cell_fullname = ".".join(starting_group.split(".")[:-1]) # remove group name
        self.cell_id = None

    def __repr__(self):
        return f"([{self.component}] Active groups: {self.active_groups})"

    def add_cell_name(self, parent_component, cell_name):
        self.cell_id = f"{self.component}[{parent_component}.{cell_name}]"
        self.parent_component = parent_component
        self.cell_name = cell_name

    def add_cell_fullname(self, cell_fullname):
        self.cell_fullname = cell_fullname

    def add_group(self, group_name):
        if group_name not in self.active_groups:
            self.active_groups.append(group_name)

    """
    Returns the active group if the component is sequential (has only one active group), otherwise throws exception
    """
    def get_active_group(self):
        if len(self.active_groups) == 0:
            return None
        elif len(self.active_groups) == 1:
            return self.active_groups[0]
        else:
            raise Exception(f'Component {self.component} is parallel! Active groups: {self.active_groups}')
    
    def get_fullname(self):
        active_group = self.get_active_group()
        if active_group is None:
            return self.cell_fullname
        else:
            return active_group

    def get_stack_depth(self):
        return self.get_fullname().count(".")

    def get_active_groups(self):
        return self.active_groups
    
    def flame_stack_string(self, main_component):
        main_shortname = main_component.split("TOP.toplevel.")[1]
        if self.component == main_shortname:
            prefix = main_component
        else:
            prefix = self.cell_id

        active_group = self.get_active_group()
        if active_group is not None:
            return prefix + ";" + active_group.split(".")[-1]
        else:
            return prefix

    def component_flame_stack_string(self, main_component):
        main_shortname = main_component.split("TOP.toplevel.")[1]
        if self.component == main_shortname:
            return main_component
        else:
            return self.component

# Computes which groups have a FSM-recorded group
def get_fsm_groups(profiled_info):
    fsm_groups = set()
    all_groups = set()
    for group_info in profiled_info:
        if group_info["name"] == "TOTAL" or group_info["component"] is None:
            continue
        all_groups.add(group_info["name"])
        if group_info["fsm_name"] is not None:
            fsm_groups.add(group_info["name"])
    return fsm_groups, all_groups

def add_elem_to_callstack(big_map, group_component, name, is_cell):
    if group_component not in big_map:
        if is_cell:
            big_map[group_component] = CallStackElement(group_component, None)
            big_map[group_component].add_cell_fullname(name)
        else:
            big_map[group_component] = CallStackElement(group_component, name)
    else:
        big_map[group_component].add_group(name)


def get_deepest_stack_element(stack_elems):
    elems_sorted = sorted(stack_elems, key=lambda k : stack_elems[k].get_stack_depth(), reverse=True)
    # NOTE: assuming sequential for now, which means (with n as the "deepest stack size"):
    # (1) There is only one element with stack depth n (this should be a group)
    # (2) There are two elements with stack depth n (a group and a cell). We want to return the cell in this case.
    if len(elems_sorted) == 1 or stack_elems[elems_sorted[0]].get_stack_depth() > stack_elems[elems_sorted[1]].get_stack_depth():
        return elems_sorted[0]
    elif stack_elems[elems_sorted[0]].get_active_groups() is None: # 0th element is a cell
        return elems_sorted[0]
    else: # 1th element has to be a cell, assuming sequential programs.
        return elems_sorted[1]

def order_callstack(main_component, cells_map, timeline):
    main_shortname = main_component.split("TOP.toplevel.")[1]
    # timeline_map has an *unordered*
    processed_trace = {}
    for i in timeline:
        if main_shortname not in timeline[i]:
            continue
        # stack = [timeline[i][main_shortname].get_active_group().split(main_component + ".")[1]] # there should always be a main component group that is active
        stack = [timeline[i][main_shortname]]
        # get the group that is deepest within the stack, then reconstruct from there
        group_component = get_deepest_stack_element(timeline[i])
        if group_component != main_shortname:
            elem_full_name = timeline[i][group_component].get_active_group()
            if elem_full_name is not None:
                after_main = elem_full_name.split(f"{main_component}.")[1]
                after_main_split = after_main.split(".")[:-1]
            else:
                # FIXME: clean this up a bit?
                elem_full_name = timeline[i][group_component].cell_fullname
                after_main = elem_full_name.split(f"{main_component}.")[1]
                after_main_split = after_main.split(".")
            prev_component = main_shortname
            for cell_name in after_main_split:
                cell_component = cells_map[prev_component][cell_name]
                timeline[i][cell_component].add_cell_name(prev_component, cell_name)
                stack.append(timeline[i][cell_component])
                prev_component = cell_component
        processed_trace[i] = stack
    return processed_trace

def create_trace(profiled_info, main_component, cells_map, fsm_groups, all_groups):
    summary = list(filter(lambda x : x["name"] == "TOTAL", profiled_info))[0]
    total_cycles = summary["total_cycles"]
    only_gt_groups = all_groups.difference(fsm_groups)
    timeline_map = {i : {} for i in range(total_cycles)}
    fsm_timeline_map = {i : {} for i in range(total_cycles)}
    group_to_gt_segments = {} # we need segment info for frequency checking
    # first iterate through all of the cells
    for cell_info in filter(lambda x : "is_cell" in x and x["is_cell"], profiled_info):
        for segment in cell_info["closed_segments"]:
            for i in range(segment["start"], segment["end"]):
                add_elem_to_callstack(fsm_timeline_map[i], cell_info["component"], cell_info["name"], True)
                add_elem_to_callstack(timeline_map[i], cell_info["component"], cell_info["name"], True)
    # next iterate through everything else
    for group_info in profiled_info:
        group_name = group_info["name"]
        if group_name == "TOTAL" or group_info["is_cell"]: # only care about actual groups
            continue
        group_component = group_info["component"]
        for segment in group_info["closed_segments"]:
            if group_info["fsm_name"] is None:
                if group_name not in group_to_gt_segments:
                    group_to_gt_segments[group_name] = {} # segment start cycle to segment end cycle
                group_to_gt_segments[group_name][segment["start"]] = segment["end"]
            for i in range(segment["start"], segment["end"]): # really janky, I wonder if there's a better way to do this?
                if group_info["fsm_name"] is not None: # FSM version
                    add_elem_to_callstack(fsm_timeline_map[i], group_component, group_name, False)
                elif group_name in only_gt_groups: # A group that isn't managed by an FSM. In which case it has to be in both FSM and GT
                    add_elem_to_callstack(fsm_timeline_map[i], group_component, group_name, False)
                    add_elem_to_callstack(timeline_map[i], group_component, group_name, False)
                else: # The ground truth info about a group managed by an FSM.
                    add_elem_to_callstack(timeline_map[i], group_component, group_name, False)

    trace = order_callstack(main_component, cells_map, timeline_map)
    fsm_trace = order_callstack(main_component, cells_map, fsm_timeline_map)

    return trace, fsm_trace, len(timeline_map)

def create_frequency_flame_graph(main_component, trace, total_cycles, frequency_flame_out):
    frequency_stacks = {}
    stack_last_cycle = ""
    for i in range(total_cycles):
        current_stack = ""
        if i in trace and len(trace[i]) != 0:
            current_stack = ";".join(map(lambda x : x.flame_stack_string(main_component), trace[i]))
        if stack_last_cycle != current_stack and stack_last_cycle.count(";") <= current_stack.count(";"): # We activated a different group, or invoked a different component!
            if current_stack not in frequency_stacks:
                frequency_stacks[current_stack] = 0
            frequency_stacks[current_stack] += 1
        stack_last_cycle = current_stack

    write_flame_graph(frequency_flame_out, frequency_stacks)

def compute_flame_stacks(trace, main_component, total_cycles):
    stacks = {}
    component_stacks = {}
    for i in trace:
        stack = ";".join(map(lambda x : x.flame_stack_string(main_component), trace[i]))
        # FIXME: really should separate out component stack
        component_stack = ";".join(map(lambda x : x.component_flame_stack_string(main_component), trace[i]))
        if stack not in stacks:
            stacks[stack] = 0
        if component_stack not in component_stacks:
            component_stacks[component_stack] = 0
        stacks[stack] += 1
        component_stacks[component_stack] += 1
    component_stacks[main_component] = total_cycles - len(trace)
    return stacks, component_stacks

# attempt to rehash the create_flame_graph to take care of stacks
def create_flame_graph(main_component, trace, fsm_trace, num_cycles, flame_out, fsm_flame_out, component_out, fsm_component_out):
    stacks, component_stacks = compute_flame_stacks(trace, main_component, num_cycles)
    write_flame_graph(flame_out, stacks)
    write_flame_graph(component_out, component_stacks)
    fsm_stacks, fsm_component_stacks = compute_flame_stacks(fsm_trace, main_component, num_cycles)
    write_flame_graph(fsm_flame_out, fsm_stacks)
    write_flame_graph(fsm_component_out, fsm_component_stacks)

def create_timeline_stacks(trace, main_component):
    events = []
    currently_active = {} # group name to beginning traceEvent entry (so end event can copy)
    ts_multiplier = 100 # some arbitrary number to multiply by so that it's easier to see in the viewer
    cell_to_stackframe_info = {main_component : (2, 1)} # (stack_number, parent_stack_number)
    stack_number_acc = 3 # To guarantee that we get unique stack numbers when we need a new one

    # Beginning and end events for main signify the overall running time (stack 1)
    main_event_details = {"name": main_component, "sf": 1, "cat": "MAIN", "pid": 1, "tid": 1}
    main_start = main_event_details.copy()
    main_start["ts"] = 0
    main_start["ph"] = "B"
    events.append(main_start)
    main_end = main_event_details.copy()
    main_end["ts"] = len(trace) * ts_multiplier
    main_end["ph"] = "E"
    events.append(main_end)
    cell_to_stackframe_info["MAIN"] = (1, None)
    cell_to_stackframe_info["TOP.toplevel.main"] = (2, 1)

    for i in trace:
        active_this_cycle = set()
        # Start from the bottom up. (easier to see parent)
        for elem in trace[i]:
            elem_full_name = elem.get_fullname()
            active_this_cycle.add(elem_full_name)
            if elem_full_name not in currently_active: # first cycle of the group. We need to figure out the stack
                elem_cell = elem.cell_fullname
                elem_shortname = elem_full_name.split(".")[-1]
                stackframe = -1 # FIXME: find the appropriate stack frame
                if elem_cell in cell_to_stackframe_info:
                    (stackframe, _) = cell_to_stackframe_info[elem_cell]
                else:
                    # Since we are iterating from the shortest to longest name (based on cell counts)
                    # The group's cell's parent *must* be in cell_to_stackframe_info
                    group_cell_parent = ".".join(elem_cell.split(".")[:-1])
                    (parent_stackframe, _) = cell_to_stackframe_info[group_cell_parent]
                    stackframe = stack_number_acc
                    stack_number_acc += 1
                    cell_to_stackframe_info[elem_cell] = (stackframe, parent_stackframe)
                start_event = {"name": f"{elem_shortname}({elem_cell})", "cat": elem.component, "ph": "B", "pid" : 1, "tid": 1, "ts": i * ts_multiplier, "sf" : stackframe}
                events.append(start_event)
                currently_active[elem_full_name] = start_event
        # Any element that was previously active but not active this cycle need to end
        for non_active_group in set(currently_active.keys()).difference(active_this_cycle):
            end_event = currently_active[non_active_group].copy()
            del currently_active[non_active_group]
            end_event["ts"] = (i) * ts_multiplier - 1
            end_event["ph"] = "E"
            events.append(end_event)
    # postprocess - add end events for all events still active by the end
    for event in currently_active:
        end_event = currently_active[event].copy()
        end_event["ts"] = (len(trace)) * ts_multiplier - 1 # only difference w the above
        end_event["ph"] = "E"
        events.append(end_event)

    # "stackFrames" field of the Trace Format JSON
    stacks = {}
    stack_category = "C"
    for cell in cell_to_stackframe_info:
        stack_id, parent_stack_id = cell_to_stackframe_info[cell]
        if parent_stack_id is None:
            stacks[stack_id] = {"name" : "MAIN", "category": stack_category}
        else:
            stacks[stack_id] = {"name" : cell, "parent": parent_stack_id, "category" : stack_category}

    return { "traceEvents": events, "stackFrames": stacks }

def create_timeline_json(trace, fsm_trace, main_component, timeline_out, fsm_timeline_out):
    timeline_json_data = create_timeline_stacks(trace, main_component)
    with open(timeline_out, "w", encoding="utf-8") as timeline_file:
        timeline_file.write(json.dumps(timeline_json_data, indent=4))
    fsm_timeline_json_data = create_timeline_stacks(fsm_trace, main_component)
    with open(fsm_timeline_out, "w", encoding="utf-8") as fsm_timeline_file:
        fsm_timeline_file.write(json.dumps(fsm_timeline_json_data, indent=4))

def write_flame_graph(flame_out, stacks):
    with open(flame_out, "w") as f:
        for stack in sorted(stacks, key=lambda k : len(k)): # main needs to come first for flame graph script to not make two boxes for main?
            f.write(f"{stack} {stacks[stack]}\n")

def build_cells_map(json_file):
    cell_json = json.load(open(json_file))
    cells_map = {}
    for component_entry in cell_json:
        inner_cells_map = {}
        for cell_entry in component_entry["cell_info"]:
            inner_cells_map[cell_entry["cell_name"]] = cell_entry["component_name"]
        cells_map[component_entry["component"]] = inner_cells_map
    return cells_map

def main(profiler_dump_file, cells_json, timeline_out, fsm_timeline_out, flame_out, fsm_flame_out, frequency_flame_out, component_out, fsm_component_out):
    profiled_info = json.load(open(profiler_dump_file, "r"))
    fsm_groups, all_groups = get_fsm_groups(profiled_info)
    # This cells_map is different from the one in parse-vcd.py
    cells_map = build_cells_map(cells_json)
    summary = list(filter(lambda x : x["name"] == "TOTAL", profiled_info))[0]
    main_component = summary["main_full_path"]
    trace, fsm_trace, num_cycles = create_trace(profiled_info, main_component, cells_map, fsm_groups, all_groups)
    create_flame_graph(main_component, trace, fsm_trace, num_cycles, flame_out, fsm_flame_out, component_out, fsm_component_out)
    create_timeline_json(trace, fsm_trace, main_component, timeline_out, fsm_timeline_out)
    create_frequency_flame_graph(main_component, trace, num_cycles, frequency_flame_out)

if __name__ == "__main__":
    if len(sys.argv) > 9:
        profiler_dump_json = sys.argv[1]
        cells_json = sys.argv[2]
        timeline_out = sys.argv[3]
        fsm_timeline_out = sys.argv[4]
        flame_out = sys.argv[5]
        fsm_flame_out = sys.argv[6]
        frequency_flame_out = sys.argv[7]
        component_flame_out = sys.argv[8]
        fsm_component_flame_out = sys.argv[9]
        main(profiler_dump_json, cells_json, timeline_out, fsm_timeline_out, flame_out, fsm_flame_out, frequency_flame_out, component_flame_out, fsm_component_flame_out)
    else:
        args_desc = [
            "PROFILER_JSON",
            "CELLS_JSON",
            "TIMELINE_VIEW_JSON",
            "FSM_TIMELINE_VIEW_JSON",
            "FLAME_GRAPH_FOLDED",
            "FSM_FLAME_GRAPH_FOLDED",
            "FREQUENCY_FLAME_GRAPH_FOLDED",
            "COMPONENT_FOLDED",
            "FSM_COMPONENT_FOLDED"
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        sys.exit(-1)
