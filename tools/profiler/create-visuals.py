"""
Takes in a dump file created by parse-vcd.py and creates files for visualization:
- *.folded files for producing flame graphs
- *.json files displaying the timeline (to be read by Perfetto UI)
"""
import json
import sys

class CallStackElement:
    """
    A component on the stack that is active at a given snapshot in time. Contains cell name and any active groups.
    (The element may not have any active groups when control is taking extra cycles for FSMs, etc.)
    starting_group is the first group that shows up. When starting_group is None, the stack simply contains the cell.
    """
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

    """
    Registers a new group active on the stack.
    """
    def add_group(self, group_name):
        if group_name not in self.active_groups:
            self.active_groups.append(group_name)

    """
    Returns the active group if the component is sequential (has only one active group), or None if the cell has no active groups.
    Throws exception if there are multiple groups active at the same time, since we are assuming sequential programs right now.
    """
    def get_active_group(self):
        if len(self.active_groups) == 0:
            return None
        elif len(self.active_groups) == 1:
            return self.active_groups[0]
        else:
            # concatenate all parallel active groups
            sorted_active_groups = list(sorted(self.active_groups))
            acc = sorted_active_groups[0]
            for group in sorted_active_groups[1:]:
                suffix = group.split(".")[-1]
                acc += "/" + suffix
            return acc
    
    """
    Returns the identifier of this stack: either the full name of the active group, or the full name of the cell if no groups are active. 
    """
    def get_fullname(self):
        active_group = self.get_active_group()
        if active_group is None:
            return self.cell_fullname
        else:
            return active_group
    
    """
    Returns the representation of the current stack for *.folded flame graph files.
    """
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

    """
    Returns the name of the stack's component.
    """
    def component_flame_stack_string(self, main_component):
        main_shortname = main_component.split("TOP.toplevel.")[1]
        if self.component == main_shortname:
            return main_component
        else:
            return self.component
        
    # other utility functions

    def get_stack_depth(self):
        return self.get_fullname().count(".")

    def add_flame_cell_name(self, parent_component, cell_name):
        self.cell_id = f"{self.component}[{parent_component}.{cell_name}]"
        self.parent_component = parent_component
        self.cell_name = cell_name

    def get_active_groups(self):
        return self.active_groups

    def add_cell_fullname(self, cell_fullname):
        self.cell_fullname = cell_fullname

"""
Computes which groups were tracked by FSMs.
"""
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

"""
Adds a new CallStackElement object for `component` to `timeline_map`
"""
def add_elem_to_callstack(timeline_map, component, name, is_cell):
    if component not in timeline_map:
        if is_cell:
            timeline_map[component] = CallStackElement(component, None)
            timeline_map[component].add_cell_fullname(name)
        else:
            timeline_map[component] = CallStackElement(component, name)
    else:
        timeline_map[component].add_group(name)

"""
Returns the stack element that is deepest in the simultaneously running sets of components
"""
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

"""
Helper method to put trace stack elements in calling order
"""
def order_trace(main_component, cells_map, timeline):
    main_shortname = main_component.split("TOP.toplevel.")[1]
    # timeline_map has an *unordered*
    processed_trace = {}
    for i in timeline:
        if main_shortname not in timeline[i]:
            continue
        stack = [timeline[i][main_shortname]]
        # get the element that is deepest within the stack, then reconstruct from there
        component = get_deepest_stack_element(timeline[i])
        if component != main_shortname:
            cell_full_name = timeline[i][component].cell_fullname
            after_main = cell_full_name.split(f"{main_component}.")[1]
            after_main_split = after_main.split(".")    
            prev_component = main_shortname
            for cell_name in after_main_split:
                cell_component = cells_map[prev_component][cell_name]
                timeline[i][cell_component].add_flame_cell_name(prev_component, cell_name)
                stack.append(timeline[i][cell_component])
                prev_component = cell_component
        processed_trace[i] = stack
    return processed_trace

"""
Constructs traces: for every cycle, what was the stack of active cells/groups?
"""
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

    trace = order_trace(main_component, cells_map, timeline_map)
    fsm_trace = order_trace(main_component, cells_map, fsm_timeline_map)

    return trace, fsm_trace, len(timeline_map)

"""
Writes a flame graph counting the number of times a group was active to `frequency_flame_out`.
"""
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

"""
Returns a representation of how many cycles each stack combination was active for.
"""
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

"""
Constructs and writes flame graphs.
"""
def create_flame_graph(main_component, trace, fsm_trace, num_cycles, flame_out, fsm_flame_out, component_out, fsm_component_out):
    stacks, component_stacks = compute_flame_stacks(trace, main_component, num_cycles)
    write_flame_graph(flame_out, stacks)
    write_flame_graph(component_out, component_stacks)
    fsm_stacks, fsm_component_stacks = compute_flame_stacks(fsm_trace, main_component, num_cycles)
    write_flame_graph(fsm_flame_out, fsm_stacks)
    write_flame_graph(fsm_component_out, fsm_component_stacks)

"""
Creates the JSON timeline representation.
"""
def create_timeline_stacks(trace, main_component):
    events = []
    currently_active = {} # group name to beginning traceEvent entry (so end event can copy)
    ts_multiplier = 100 # some arbitrary number to multiply by so that it's easier to see in the viewer
    cell_to_stackframe_info = {main_component : (2, 1)} # (stack_number, parent_stack_number)
    stack_number_acc = 3 # To guarantee that we get unique stack numbers when we need a new one

    cell_to_stackframe_info["MAIN"] = (1, None)
    cell_to_stackframe_info["TOP.toplevel"] = (2, 1)

    for i in trace:
        active_this_cycle = set()
        # Start from the bottom up. Parent is the previous stack!
        parent = "MAIN"
        for elem in trace[i]:
            # we want an entry for the cell and the group, if the stack entry has a group in it.
            # cell
            cell_name = elem.cell_fullname
            active_this_cycle.add(cell_name)
            if cell_name not in currently_active: # first cycle of the cell.
                # get the stackframe
                if cell_name not in cell_to_stackframe_info:
                    (parent_stackframe, _) = cell_to_stackframe_info[parent] # the parent better have been registered by now
                    cell_to_stackframe_info[cell_name] = (stack_number_acc, parent_stackframe)
                    stack_number_acc += 1
                (cell_stackframe, _) = cell_to_stackframe_info[cell_name]
                # add start event of this cell
                start_event = {"name": cell_name.split(".")[-1], "cat": "cell", "ph": "B", "pid" : 1, "tid": 1, "ts": i * ts_multiplier, "sf" : cell_stackframe}
                events.append(start_event)
                currently_active[cell_name] = start_event
            # add a group if one is active
            group_name = elem.get_active_group()
            if group_name is not None:
                active_this_cycle.add(group_name)
                if group_name not in currently_active:
                    # get the stackframe
                    if group_name not in cell_to_stackframe_info:
                        cell_to_stackframe_info[group_name] = (stack_number_acc, cell_stackframe) # parent of the group is the cell
                        stack_number_acc += 1
                    (group_stackframe, _) = cell_to_stackframe_info[group_name]
                    # add start event of this group
                    start_event = {"name": group_name.split(".")[-1], "cat": "group", "ph": "B", "pid" : 1, "tid": 1, "ts": i * ts_multiplier, "sf" : group_stackframe}
                    events.append(start_event)
                    currently_active[group_name] = start_event
                parent = group_name # the next cell's parent will be this group.
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

"""
Wrapper function for constructing and writing a timeline representation
"""
def create_timeline_json(trace, fsm_trace, main_component, timeline_out, fsm_timeline_out):
    timeline_json_data = create_timeline_stacks(trace, main_component)
    with open(timeline_out, "w", encoding="utf-8") as timeline_file:
        timeline_file.write(json.dumps(timeline_json_data, indent=4))
    fsm_timeline_json_data = create_timeline_stacks(fsm_trace, main_component)
    with open(fsm_timeline_out, "w", encoding="utf-8") as fsm_timeline_file:
        fsm_timeline_file.write(json.dumps(fsm_timeline_json_data, indent=4))

"""
Helper function to output the *.folded file for flame graphs.
"""
def write_flame_graph(flame_out, stacks):
    with open(flame_out, "w") as f:
        for stack in sorted(stacks, key=lambda k : len(k)): # main needs to come first for flame graph script to not make two boxes for main?
            f.write(f"{stack} {stacks[stack]}\n")

"""
Helper function to process the cells_json input.
"""
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
