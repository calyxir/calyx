import copy
import json
import os
import sys
import vcdvcd

DELIMITER = "__"
INVISIBLE = "gray"
TREE_PICTURE_LIMIT=300
SCALED_FLAME_MULTIPLIER=1000

def remove_size_from_name(name: str) -> str:
    """ changes e.g. "state[2:0]" to "state" """
    return name.split('[')[0]

class ProfilingInfo:
    def __init__(self, probe_info, probe_type):
        self.name = name
        self.callsite = callsite # "official" call site. Should only be used for probes?
        self.component = component
        self.shortname = self.name.split(".")[-1]
        self.closed_segments = [] # Segments will be (start_time, end_time)
        self.current_segment = None
        self.total_cycles = 0
        self.is_cell = is_cell
    
    def flame_repr(self):
        if self.is_cell:
            return self.name
        else:
            return self.shortname

    def __repr__ (self):
        if self.is_cell:
            header = f"[Cell][{self.callsite}] {self.name}" # FIXME: fix this later
        else:
            header = f"[{self.component}][{self.callsite}] {self.name}"
        return header

    def id(self):
        return f"{self.name}{DELIMITER}{self.component}"

class VCDConverter(vcdvcd.StreamParserCallbacks):
    def __init__(self, main_component, cells_to_components):
        super().__init__()
        self.main_component = main_component
        self.cells_to_components = cells_to_components
        # Documenting other fields for reference
        # signal_id_to_names
        self.timestamps_to_events = {}

    def enddefinitions(self, vcd, signals, cur_sig_vals):
        # convert references to list and sort by name
        refs = [(k, v) for k, v in vcd.references_to_ids.items()]
        refs = sorted(refs, key=lambda e: e[0])
        names = [remove_size_from_name(e[0]) for e in refs]
        signal_id_dict = {sid : [] for sid in vcd.references_to_ids.values()} # one id can map to multiple signal names since wires are connected

        clock_name = f"{self.main_component}.clk"
        if clock_name not in names:
            print("Can't find the clock? Exiting...")
            sys.exit(1)
        signal_id_dict[vcd.references_to_ids[clock_name]] = [clock_name]

        # get go and done for cells (the signals are exactly {cell}.go and {cell}.done)
        for cell in self.cells_to_components.keys():
            cell_go = cell + ".go"
            cell_done = cell + ".done"
            if cell_go not in vcd.references_to_ids:
                print(f"Not accounting for cell {cell} (probably combinational)")
                continue
            signal_id_dict[vcd.references_to_ids[cell_go]].append(cell_go)
            signal_id_dict[vcd.references_to_ids[cell_done]].append(cell_done)

        for name, sid in refs:
            if "probe_out" in name:
                signal_id_dict[sid].append(name)

        # don't need to check for signal ids that don't pertain to signals we're interested in
        self.signal_id_to_names = {k:v for k,v in signal_id_dict.items() if len(v) > 0}
    
    def value(self, vcd, time, value, identifier_code, cur_sig_vals):
        # ignore all signals we don't care about
        if identifier_code not in self.signal_id_to_names:
            return
        
        signal_names = self.signal_id_to_names[identifier_code]
        int_value = int(value, 2)

        if time not in self.timestamps_to_events:
            self.timestamps_to_events[time] = []

        for signal_name in signal_names:
            event = {"signal": signal_name, "value": int_value}
            self.timestamps_to_events[time].append(event)
    
    # Postprocess data mapping timestamps to events (signal changes)
    # We have to postprocess instead of processing signals in a stream because
    # signal changes that happen at the same time as a clock tick might be recorded
    # *before* or *after* the clock change on the VCD file (hence why we can't process
    # everything within a stream if we wanted to be precise)
    def postprocess(self):
        clock_name = f"{self.main_component}.clk"
        clock_cycles = -1 # will be 0 on the 0th cycle
        started = False
        cell_active = set()
        group_active = set()
        structural_enable_active = set()
        cell_enable_active = set()
        primitive_enable = set()

        probe_labels_to_sets = {"group_probe_out": group_active, "se_probe_out": structural_enable_active, "cell_probe_out": cell_enable_active, "primitive_probe_out" : primitive_enable}

        self.timeline_map = {} # cycle --> {"cell-active": set(), "group-active": set(), "structural-enable": set(), "cell-enable": set(), "primitive-enable": set()}
        
        for ts in self.timestamps_to_events:
            events = self.timestamps_to_events[ts]
            started = started or [x for x in events if x["signal"] == f"{self.main_component}.go" and x["value"] == 1]
            if not started: # only start counting when main component is on.
                continue
            # checking whether the timestamp has a rising edge
            if {"signal": clock_name, "value": 1} in events:
                clock_cycles += 1
            for event in events:
                # check probe and cell signals to update currently active entities.
                signal_name = event["signal"]
                value = event["value"]
                if signal_name.endswith(".go") and value == 1: # cells have .go and .done
                    cell = signal_name.split(".go")[0]
                    cell_active.add(cell)
                if signal_name.endswith(".done") and value == 1:
                    cell = signal_name.split(".done")[0]
                    cell_active.remove(cell)
                for probe_label in probe_labels_to_sets:
                    cutoff = f"_{probe_label}"
                    if cutoff in signal_name:
                        probe_info = signal_name.split(cutoff)[0]
                        if value == 1:
                            probe_labels_to_sets[probe_label].add(probe_info)
                        elif value == 0:
                            probe_labels_to_sets[probe_label].remove(probe_info)
                # add currently active entities to the timeline map
                self.timeline_map[clock_cycles] = copy.deepcopy(probe_labels_to_sets)
                # so awkward... add cell_active because it wasn't a probe.
                self.timeline_map[clock_cycles]["cell_active"] = cell_active.copy()

        self.clock_cycles = clock_cycles

# Generates a list of all of the components to potential cell names
# `prefix` is the cell's "path" (ex. for a cell "my_cell" defined in "main", the prefix would be "TOP.toplevel.main")
# The initial value of curr_component should be the top level/main component
def build_components_to_cells(prefix, curr_component, cells_to_components, components_to_cells):
    for (cell, cell_component) in cells_to_components[curr_component].items():
        if cell_component not in components_to_cells:
            components_to_cells[cell_component] = [f"{prefix}.{cell}"]
        else:
            components_to_cells[cell_component].append(f"{prefix}.{cell}")
        build_components_to_cells(prefix + f".{cell}", cell_component, cells_to_components, components_to_cells)

# Reads json generated by component-cells backend to produce a mapping from all components
# to cell names they could have.
def read_component_cell_names_json(json_file):
    cell_json = json.load(open(json_file))
    # For each component, contains a map from each cell name to its corresponding component
    # component name --> { cell name --> component name }
    cells_to_components = {}
    main_component = ""
    for curr_component_entry in cell_json:
        cell_map = {} # mapping cell names to component names for all cells in the current component
        if curr_component_entry["is_main_component"]:
            main_component = curr_component_entry["component"]
        for cell_info in curr_component_entry["cell_info"]:
            cell_map[cell_info["cell_name"]] = cell_info["component_name"]
        cells_to_components[curr_component_entry["component"]] = cell_map
    full_main_component = f"TOP.toplevel.{main_component}"
    components_to_cells = {main_component : [full_main_component]} # come up with a better name for this
    build_components_to_cells(full_main_component, main_component, cells_to_components, components_to_cells)
    full_cell_names_to_components = {}
    for component in components_to_cells:
        for cell in components_to_cells[component]:
            full_cell_names_to_components[cell] = component

    return full_main_component, full_cell_names_to_components

def create_traces(timeline_map, cells_to_components, main_component):

    trace = {i : [] for i in range(timeline_map.keys())}
    # now, we need to figure out the sets of traces
    for i in timeline_map:
        # intermediate processing; see if I can skip this step...
        

        call_stack_probes_info = timeline_map[i]["se_probe_out"]
        primitive_probes_info = timeline_map[i]["primitive_probe_out"]
        cell_caller_probes_info = timeline_map[i][""]
        parents = set() # keeping track of entities that are parents of other entities
        i_mapping = {} # each unique group inv mapping to its stack. the "group" should be the last item on each stack
        i_mapping[main_component] = [main_component.split(".")[-1]]

        cell_worklist = [main_component] # FIXME: maybe remove the hardcoding?
        while len(cell_worklist) > 0:
            


            current_cell = cell_worklist.pop()
            current_component = cells_to_components[current_cell]
            covered_units_in_component = set() # collect all of the units we've covered.
            # catch all active units that are groups in this component.
            units_to_cover = set(filter(lambda unit: not unit.is_cell and unit.component == current_component, timeline_map[i]))
            # find all enables from control. these are all units that either (1) don't have any maps in call_stack_probes_info, or (2) have no active parent calls in call_stack_probes_info
            for active_unit in units_to_cover:
                if active_unit.is_cell: # skip cells for now as we're considering only single component programs
                    continue
                if active_unit.id() not in call_stack_probes_info: # no maps in call_stack_probes_info
                    i_mapping[active_unit.name] = i_mapping[current_cell] + [active_unit.shortname]
                    parents.add(current_cell)
                    covered_units_in_component.add(active_unit.name)
                else:
                    # loop through all parents and see if any of them are active
                    contains_active_parent = False
                    for parent, call_probe_info in call_stack_probes_info[active_unit.id()].items():
                        if call_probe_info.is_active_at_cycle(i):
                            contains_active_parent = True
                            break
                    if not contains_active_parent:
                        i_mapping[active_unit.name] = i_mapping[current_cell] + [active_unit.shortname]
                        parents.add(current_cell)
                        covered_units_in_component.add(active_unit.name)
            # get all of the other active units
            while len(covered_units_in_component) < len(units_to_cover):
                # loop through all other elements to figure out parent child info
                for active_unit in units_to_cover:
                    if active_unit.is_cell or active_unit.name in i_mapping:
                        continue
                    for parent, call_probe_info in call_stack_probes_info[active_unit.id()].items():
                        if f"{main_component}.{parent}" in i_mapping: # we can directly build on top of the parent
                            i_mapping[active_unit.name] = i_mapping[f"{current_cell}.{parent}"] + [active_unit.shortname]
                            covered_units_in_component.add(active_unit.name)
                        parents.add(f"{current_cell}.{parent}")
            # get primitives.
            # primitives probe info: parent --> {primitive_name --> info}. So we want to loop through all of the groups.
            for active_unit in units_to_cover:
                if active_unit.id() in primitive_probes_info: # this group is calling a primitive.
                    for primitive in primitive_probes_info[active_unit.id()]:
                        primitive_info = primitive_probes_info[active_unit.id()][primitive]
                        i_mapping[primitive_info.name] = i_mapping[active_unit.name] + [f"{primitive_info.shortname} (primitive)"]
                        parents.add(active_unit.name)
            # by this point, we should have covered all groups in the same component...
            # now we need to construct stacks for any cells that are called from a group in the current component.
            # collect caller ids in cell_caller_probes_info that belong to our component
            cell_invoker_ids = list(filter(lambda x : x.split(DELIMITER)[1] == current_component, cell_caller_probes_info))
            for cell_invoker_id in cell_invoker_ids:
                cell_invoker = cell_invoker_id.split(DELIMITER)[0]
                # iterate through all of the cells that the group invokes
                for invoked_cell_name in cell_caller_probes_info[cell_invoker_id]:
                    cell_calling_probe = cell_caller_probes_info[cell_invoker_id][invoked_cell_name]
                    cell_active_probe = active_element_probes_info[invoked_cell_name]
                    if cell_calling_probe.is_active_at_cycle(i) and cell_active_probe.is_active_at_cycle(i):
                        cell_worklist.append(cell_active_probe.name)
                        # invoker group is the parent of the cell.
                        cell_component = cells_to_components[cell_active_probe.name]
                        i_mapping[cell_active_probe.name] = i_mapping[f"{current_cell}.{cell_invoker}"] + [f"{cell_active_probe.shortname} [{cell_component}]"]
                        parents.add(f"{current_cell}.{cell_invoker}")

        # Only retain paths that lead to leaf nodes.
        for elem in i_mapping:
            if elem not in parents:
                trace[i].append(i_mapping[elem])
        
    for i in trace:
        print(i)
        for stack in trace[i]:
            print(f"\t{stack}")

    return trace

"""
Creates a tree that encapsulates all stacks that occur within the program.
"""
def create_tree(timeline_map):
    node_id_acc = 0
    tree_dict = {} # node id --> node name
    path_dict = {} # stack list string --> list of node ids
    path_prefixes_dict = {} # stack list string --> list of node ids
    stack_list = []
    # collect all of the stacks from the list. (i.e. "flatten" the timeline map values.)
    for sl in timeline_map.values():
        for s in sl:
            if s not in stack_list:
                stack_list.append(s)
    stack_list.sort(key=len)
    for stack in stack_list:
        stack_len = len(stack)
        id_path_list = []
        prefix = ""
        # obtain the longest prefix of the current stack. Everything after the prefix is a new stack element.
        for i in range(1, stack_len+1):
            attempted_prefix = ";".join(stack[0:stack_len-i])
            if attempted_prefix in path_prefixes_dict:
                prefix = attempted_prefix
                id_path_list = list(path_prefixes_dict[prefix])
                break
        # create nodes
        if prefix != "":
            new_nodes = stack[stack_len - i:]
            new_prefix = prefix
        else:
            new_nodes = stack
            new_prefix = ""
        for elem in new_nodes:
            if new_prefix == "":
                new_prefix = elem
            else:
                new_prefix += f";{elem}"
            tree_dict[node_id_acc] = elem
            id_path_list.append(node_id_acc)
            path_prefixes_dict[new_prefix] = list(id_path_list)
            node_id_acc += 1
        path_dict[new_prefix] = id_path_list

    return tree_dict, path_dict

def create_tree_rankings(trace, tree_dict, path_dict, path_to_edges, all_edges, dot_out_dir):
    stack_list_str_to_used_nodes = {}
    stack_list_str_to_used_edges = {}
    stack_list_str_to_cycles = {}
    all_nodes = set(tree_dict.keys())

    # accumulating counts
    for i in trace:
        stack_list_str = str(trace[i])
        if stack_list_str in stack_list_str_to_cycles:
            stack_list_str_to_cycles[stack_list_str].append(i)
            continue
        stack_list_str_to_cycles[stack_list_str] = [i]
        used_nodes = set()
        used_edges = set()

        for stack in trace[i]:
            stack_id = ";".join(stack)
            for node_id in path_dict[stack_id]:
                used_nodes.add(node_id)
            for edge in path_to_edges[stack_id]:
                used_edges.add(edge)
        stack_list_str_to_used_nodes[stack_list_str] = used_nodes
        stack_list_str_to_used_edges[stack_list_str] = used_edges

    sorted_stack_list_items = sorted(stack_list_str_to_cycles.items(), key=(lambda item : len(item[1])), reverse=True)
    acc = 0
    rankings_out = open(os.path.join(dot_out_dir, "rankings.txt"), "w")
    rankings_out.write("Rank,#Cycles,Cycles-list\n")
    for (stack_list_str, cycles) in sorted_stack_list_items:
        if acc == 5:
            break
        acc += 1
        # draw the tree
        fpath = os.path.join(dot_out_dir, f"rank{acc}.dot")
        with open(fpath, "w") as f:
            f.write("digraph rank" + str(acc) + " {\n")
            # declare nodes.
            for node in all_nodes:
                if node in stack_list_str_to_used_nodes[stack_list_str]:
                    f.write(f'\t{node} [label="{tree_dict[node]}"];\n')
                else:
                    f.write(f'\t{node} [label="{tree_dict[node]}",color="{INVISIBLE}",fontcolor="{INVISIBLE}"];\n')
            # write all edges.
            for edge in all_edges:
                if edge in stack_list_str_to_used_edges[stack_list_str]:
                    f.write(f'\t{edge} ; \n')
                else:
                    f.write(f'\t{edge} [color="{INVISIBLE}"]; \n')
            f.write("}")

        # should write to a txt file what
        rankings_out.write(f"{acc},{len(cycles)},{';'.join(str(c) for c in cycles)}\n")


# one tree to summarize the entire execution.
def create_aggregate_tree(timeline_map, out_dir, tree_dict, path_dict):
    # NOTE: probably can create this on the fly, but my brain is better suited for postprocessing.
    path_to_edges, all_edges = create_edge_dict(path_dict)

    leaf_nodes_dict = {node_id: 0 for node_id in tree_dict} # how many times was this node a leaf?
    edges_dict = {} # how many times was this edge active?

    for stack_list in timeline_map.values():
        edges_this_cycle = set()
        leaves_this_cycle = set()
        for stack in stack_list:
            stack_id = ";".join(stack)
            # record the leaf node. ignore all primitives as I think we care more about the group that called the primitive (up to debate)
            leaf_node = path_dict[stack_id][-1]
            if "primitive" in tree_dict[leaf_node]:
                leaf_node = path_dict[stack_id][-2]
            if leaf_node not in leaves_this_cycle:
                leaf_nodes_dict[leaf_node] += 1
                leaves_this_cycle.add(leaf_node)
            for edge in path_to_edges[stack_id]:
                if edge not in edges_this_cycle:
                    if edge not in edges_dict:
                        edges_dict[edge] = 1
                    else:
                        edges_dict[edge] += 1
                    edges_this_cycle.add(edge)
    
    # write the tree
    if not os.path.exists(out_dir):
        os.mkdir(out_dir)
    with open(os.path.join(out_dir, "aggregate.dot"), "w") as f:
        f.write("digraph aggregate {\n")
        # declare nodes
        for node in leaf_nodes_dict:
            if "primitive" in tree_dict[node]:
                f.write(f'\t{node} [label="{tree_dict[node]}"];\n')
            else:
                f.write(f'\t{node} [label="{tree_dict[node]} ({leaf_nodes_dict[node]})"];\n')
        # write edges with labels
        for edge in edges_dict:
            f.write(f'\t{edge} [label="{edges_dict[edge]}"]; \n')
        f.write("}")

def create_path_dot_str_dict(path_dict):
    path_to_dot_str = {} # stack list string --> stack path representation on dot file.

    for path_id in path_dict:
        path = path_dict[path_id]
        path_acc = ""
        for node_id in path[0:-1]:
            path_acc += f'{node_id} -> '
        path_acc += f'{path[-1]}'
        path_to_dot_str[path_id] = path_acc

    return path_to_dot_str

def create_edge_dict(path_dict):
    path_to_edges = {} # stack list string --> [edge string representation]
    all_edges = set()

    for path_id in path_dict:
        path = path_dict[path_id]
        edge_set = []
        for i in range(len(path)-1):
            edge = f"{path[i]} -> {path[i+1]}"
            edge_set.append(edge)
            all_edges.add(edge)
        path_to_edges[path_id] = edge_set

    return path_to_edges, list(sorted(all_edges))

# create a tree where we divide cycles via par arms
def compute_scaled_flame(timeline_map):
    stacks = {}
    for i in timeline_map:
        num_stacks = len(timeline_map[i])
        cycle_slice = round(1 / num_stacks, 3)
        last_cycle_slice = 1 - (cycle_slice * (num_stacks - 1))
        acc = 0
        for stack_list in timeline_map[i]:
            stack_id = ";".join(stack_list)
            slice_to_add = cycle_slice if acc < num_stacks - 1 else last_cycle_slice
            if stack_id not in stacks:
                stacks[stack_id] = slice_to_add * SCALED_FLAME_MULTIPLIER
            else:
                stacks[stack_id] += slice_to_add * SCALED_FLAME_MULTIPLIER
            acc += 1
            
    return stacks

def create_flame_groups(timeline_map, flame_out_file, flames_out_dir):
    if not os.path.exists(flames_out_dir):
        os.mkdir(flames_out_dir)    
    
    # make flame graph folded file
    stacks = {} # stack to number of cycles
    for i in timeline_map:
        for stack_list in timeline_map[i]:
            stack_id = ";".join(stack_list)
            if stack_id not in stacks:
                stacks[stack_id] = 1
            else:
                stacks[stack_id] += 1
    
    with open(flame_out_file, "w") as flame_out:
        for stack in stacks:
            flame_out.write(f"{stack} {stacks[stack]}\n")

    scaled_stacks = compute_scaled_flame(timeline_map)
    with open(os.path.join(flames_out_dir, "scaled-flame.folded"), "w") as div_flame_out:
        for stack in scaled_stacks:
            div_flame_out.write(f"{stack} {scaled_stacks[stack]}\n")

def create_slideshow_dot(timeline_map, dot_out_dir, flame_out_file, flames_out_dir):

    if not os.path.exists(dot_out_dir):
        os.mkdir(dot_out_dir)

    # probably wise to not have a billion dot files.
    if len(timeline_map) > TREE_PICTURE_LIMIT:
        print(f"Simulation exceeds {TREE_PICTURE_LIMIT} cycles, skipping trees...")
        return
    tree_dict, path_dict = create_tree(timeline_map)
    path_to_edges, all_edges = create_edge_dict(path_dict)

    for i in timeline_map:
        used_edges = {}
        used_paths = set()
        used_nodes = set()
        all_nodes = set(tree_dict.keys())
        # figure out what nodes are used and what nodes aren't used
        for stack in timeline_map[i]:
            stack_id = ";".join(stack)
            used_paths.add(stack_id)
            for node_id in path_dict[stack_id]:
                used_nodes.add(node_id)
            for edge in path_to_edges[stack_id]:
                if edge not in used_edges:
                    used_edges[edge] = 1
                else:
                    used_edges[edge] += 1

        fpath = os.path.join(dot_out_dir, f"cycle{i}.dot")
        with open(fpath, "w") as f:
            f.write("digraph cycle" + str(i) + " {\n")
            # declare nodes.
            for node in all_nodes:
                if node in used_nodes:
                    f.write(f'\t{node} [label="{tree_dict[node]}"];\n')
                else:
                    f.write(f'\t{node} [label="{tree_dict[node]}",color="{INVISIBLE}",fontcolor="{INVISIBLE}"];\n')
            # write all edges.
            for edge in all_edges:
                if edge in used_edges.keys():
                    f.write(f'\t{edge} ; \n')
                else:
                    f.write(f'\t{edge} [color="{INVISIBLE}"]; \n')
            f.write("}")

def main(vcd_filename, cells_json_file, dot_out_dir, flame_out, flames_out_dir):
    main_component, cells_to_components = read_component_cell_names_json(cells_json_file)
    converter = VCDConverter(main_component, cells_to_components)
    vcdvcd.VCDVCD(vcd_filename, callbacks=converter)
    converter.postprocess()

    for i in converter.timeline_map:
        if i < 5:
            print(i)
            print(converter.timeline_map[i])

    # trace = create_traces(converter.timeline_map, converter.clock_cycles, cells_to_components, main_component)

    # tree_dict, path_dict = create_tree(trace)
    # path_to_edges, all_edges = create_edge_dict(path_dict)

    # create_aggregate_tree(trace, dot_out_dir, tree_dict, path_dict)
    # create_tree_rankings(trace, tree_dict, path_dict, path_to_edges, all_edges, dot_out_dir)
    # create_flame_groups(trace, flame_out, flames_out_dir)


if __name__ == "__main__":
    if len(sys.argv) > 5:
        vcd_filename = sys.argv[1]
        cells_json = sys.argv[2]
        dot_out_dir = sys.argv[3]
        flame_out = sys.argv[4]
        flames_out_dir = sys.argv[5] # tmp folder until I figure out how to get multiple outputs from fud2
        main(vcd_filename, cells_json, dot_out_dir, flame_out, flames_out_dir)
    else:
        args_desc = [
            "VCD_FILE",
            "CELLS_JSON",
            "DOT_FILE_DIR",
            "FLAME_OUT",
            "FLAME_OUT_DIR"
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        print("CELLS_JSON: Run the `component_cells` tool")
        sys.exit(-1)
