from datetime import datetime
import csv
import json
import os
import sys
import vcdvcd

DELIMITER = "___"
INVISIBLE = "gray"
ACTIVE_CELL_COLOR = "pink"
ACTIVE_GROUP_COLOR = "mediumspringgreen"
ACTIVE_PRIMITIVE_COLOR = "orange"
TREE_PICTURE_LIMIT = 300
SCALED_FLAME_MULTIPLIER = (
    1000  # [flame graph] multiplier so scaled flame graph will not round up.
)
ts_multiplier = 1  # [timeline view] ms on perfetto UI that resembles a single cycle
JSON_INDENT = "    "  # [timeline view] indentation for generating JSON on the fly
num_timeline_events = 0  # [timeline view] recording how many events have happened


def remove_size_from_name(name: str) -> str:
    """changes e.g. "state[2:0]" to "state" """
    return name.split("[")[0]


def create_cycle_trace(
    info_this_cycle, cells_to_components, main_component, include_primitives
):
    stacks_this_cycle = []
    parents = set()  # keeping track of entities that are parents of other entities
    i_mapping = {}  # each unique group inv mapping to its stack. the "group" should be the last item on each stack
    i_mapping[main_component] = [main_component.split(".")[-1]]
    cell_worklist = [main_component]
    while len(cell_worklist) > 0:
        current_cell = cell_worklist.pop()
        covered_units_in_component = set()  # collect all of the units we've covered.
        # catch all active units that are groups in this component.
        units_to_cover = (
            info_this_cycle["group-active"][current_cell]
            if current_cell in info_this_cycle["group-active"]
            else set()
        )
        structural_enables = (
            info_this_cycle["structural-enable"][current_cell]
            if current_cell in info_this_cycle["structural-enable"]
            else set()
        )
        primitive_enables = (
            info_this_cycle["primitive-enable"][current_cell]
            if current_cell in info_this_cycle["primitive-enable"]
            else set()
        )
        cell_invokes = (
            info_this_cycle["cell-invoke"][current_cell]
            if current_cell in info_this_cycle["cell-invoke"]
            else set()
        )
        # find all enables from control. these are all units that either (1) don't have any maps in call_stack_probes_info, or (2) have no active parent calls in call_stack_probes_info
        for active_unit in units_to_cover:
            shortname = active_unit.split(".")[-1]
            if active_unit not in structural_enables:
                i_mapping[active_unit] = i_mapping[current_cell] + [shortname]
                parents.add(current_cell)
                covered_units_in_component.add(active_unit)
        # get all of the other active units
        while len(covered_units_in_component) < len(units_to_cover):
            # loop through all other elements to figure out parent child info
            for active_unit in units_to_cover:
                shortname = active_unit.split(".")[-1]
                if active_unit in i_mapping:
                    continue
                for parent_group in structural_enables[active_unit]:
                    parent = f"{current_cell}.{parent_group}"
                    if parent in i_mapping:
                        i_mapping[active_unit] = i_mapping[parent] + [shortname]
                        covered_units_in_component.add(active_unit)
                        parents.add(parent)
        # get primitives if requested.
        if include_primitives:
            for primitive_parent_group in primitive_enables:
                for primitive_name in primitive_enables[primitive_parent_group]:
                    primitive_parent = f"{current_cell}.{primitive_parent_group}"
                    primitive_shortname = primitive_name.split(".")[-1]
                    i_mapping[primitive_name] = i_mapping[primitive_parent] + [
                        f"{primitive_shortname} (primitive)"
                    ]
                    parents.add(primitive_parent)
        # by this point, we should have covered all groups in the same component...
        # now we need to construct stacks for any cells that are called from a group in the current component.
        for cell_invoker_group in cell_invokes:
            for invoked_cell in cell_invokes[cell_invoker_group]:
                if invoked_cell in info_this_cycle["cell-active"]:
                    cell_shortname = invoked_cell.split(".")[-1]
                    cell_worklist.append(invoked_cell)
                    cell_component = cells_to_components[invoked_cell]
                    parent = f"{current_cell}.{cell_invoker_group}"
                    i_mapping[invoked_cell] = i_mapping[parent] + [
                        f"{cell_shortname} [{cell_component}]"
                    ]
                    parents.add(parent)
    # Only retain paths that lead to leaf nodes.
    for elem in i_mapping:
        if elem not in parents:
            stacks_this_cycle.append(i_mapping[elem])

    return stacks_this_cycle


class VCDConverter(vcdvcd.StreamParserCallbacks):
    def __init__(
        self,
        main_component,
        cells_to_components,
        fsms,
        fsm_events,
        par_groups,
        par_done_regs,
    ):
        super().__init__()
        self.main_shortname = main_component
        self.cells_to_components = cells_to_components
        self.timestamps_to_events = {}  # timestamps to
        self.timestamps_to_clock_cycles = {}
        self.timestamps_to_control_reg_changes = {}
        self.timestamps_to_control_group_events = {}
        self.fsms = fsms
        self.partial_fsm_events = fsm_events
        self.par_done_regs = par_done_regs
        self.par_groups = par_groups

    def enddefinitions(self, vcd, signals, cur_sig_vals):
        # convert references to list and sort by name
        refs = [(k, v) for k, v in vcd.references_to_ids.items()]
        refs = sorted(refs, key=lambda e: e[0])
        names = [remove_size_from_name(e[0]) for e in refs]
        signal_id_dict = {
            sid: [] for sid in vcd.references_to_ids.values()
        }  # one id can map to multiple signal names since wires are connected
        tdcc_signal_id_to_names = {
            sid: [] for sid in vcd.references_to_ids.values()
        }  # same as signal_id_dict, but just the registers that manage control (fsm, pd)
        control_signal_id_to_names = {
            sid: [] for sid in vcd.references_to_ids.values()
        }  # same as signal_id_dict, but just groups that manage control (only par for now, can also consider tdcc)

        clock_filter = list(
            filter(lambda x: x.endswith(f"{self.main_shortname}.clk"), names)
        )
        if len(clock_filter) > 1:
            print(f"Found multiple clocks: {clock_filter} Exiting...")
            sys.exit(1)
        elif len(clock_filter) == 0:
            print("Can't find the clock? Exiting...")
            sys.exit(1)
        clock_name = clock_filter[0]
        # Depending on the simulator + OS, we may get different prefixes before the name
        # of the main component.
        self.signal_prefix = clock_name.split(f".{self.main_shortname}")[0]
        self.main_component = f"{self.signal_prefix}.{self.main_shortname}"
        signal_id_dict[vcd.references_to_ids[clock_name]] = [clock_name]

        # get go and done for cells (the signals are exactly {cell}.go and {cell}.done)
        for cell_suffix in list(self.cells_to_components.keys()):
            cell = f"{self.signal_prefix}.{cell_suffix}"
            cell_go = cell + ".go"
            cell_done = cell + ".done"
            if cell_go not in vcd.references_to_ids:
                print(f"Not accounting for cell {cell} (probably combinational)")
                continue
            signal_id_dict[vcd.references_to_ids[cell_go]].append(cell_go)
            signal_id_dict[vcd.references_to_ids[cell_done]].append(cell_done)
            # replace the old key (cell_suffix) with the fully qualified cell name
            self.cells_to_components[cell] = self.cells_to_components[cell_suffix]
            del self.cells_to_components[cell_suffix]
        # update fsms, par done registers, par groups with fully qualified names
        self.fsms = {f"{self.signal_prefix}.{fsm}" for fsm in self.fsms}
        self.partial_fsm_events = {
            f"{self.signal_prefix}.{fsm}": self.partial_fsm_events[fsm]
            for fsm in self.partial_fsm_events
        }
        self.par_done_regs = {f"{self.signal_prefix}.{pd}" for pd in self.par_done_regs}
        self.par_groups = {
            f"{self.signal_prefix}.{par_group}" for par_group in self.par_groups
        }

        for name, sid in refs:
            if "probe_out" in name:
                signal_id_dict[sid].append(name)
            for fsm in self.fsms:
                if name.startswith(f"{fsm}.out["):
                    signal_id_dict[sid].append(name)
                if name.startswith(f"{fsm}.write_en") or name.startswith(f"{fsm}.in"):
                    tdcc_signal_id_to_names[sid].append(name)
            for par_done_reg in self.par_done_regs:
                if (
                    name.startswith(f"{par_done_reg}.in")
                    or name == f"{par_done_reg}.write_en"
                ):
                    tdcc_signal_id_to_names[sid].append(name)
            for par_group_name in self.par_groups:
                if name == f"{par_group_name}_go_out":
                    control_signal_id_to_names[sid].append(name)
        del self.par_groups

        # don't need to check for signal ids that don't pertain to signals we're interested in
        self.signal_id_to_names = {
            k: v for k, v in signal_id_dict.items() if len(v) > 0
        }
        self.tdcc_signal_id_to_names = {
            k: v for k, v in tdcc_signal_id_to_names.items() if len(v) > 0
        }
        self.control_signal_id_to_names = {
            k: v for k, v in control_signal_id_to_names.items() if len(v) > 0
        }

    def value(self, vcd, time, value, identifier_code, cur_sig_vals):
        int_value = int(value, 2)
        if identifier_code in self.signal_id_to_names:
            signal_names = self.signal_id_to_names[identifier_code]

            for signal_name in signal_names:
                if (
                    signal_name == f"{self.main_component}.clk" and int_value == 0
                ):  # ignore falling edges
                    continue
                event = {"signal": signal_name, "value": int_value}
                if time not in self.timestamps_to_events:
                    self.timestamps_to_events[time] = [event]
                else:
                    self.timestamps_to_events[time].append(event)
        if identifier_code in self.control_signal_id_to_names:
            signal_names = self.control_signal_id_to_names[identifier_code]
            for signal_name in signal_names:
                clean_signal_name = (
                    remove_size_from_name(signal_name)
                    .split("_go_out")[0]
                    .replace(self.signal_prefix + ".", "")
                )
                event = {"group": clean_signal_name, "value": int_value}
                if time not in self.timestamps_to_control_group_events:
                    self.timestamps_to_control_group_events[time] = [event]
                else:
                    self.timestamps_to_control_group_events[time].append(event)
        if identifier_code in self.tdcc_signal_id_to_names:
            signal_names = self.tdcc_signal_id_to_names[identifier_code]

            for signal_name in signal_names:
                clean_signal_name = remove_size_from_name(signal_name)
                if time not in self.timestamps_to_control_reg_changes:
                    self.timestamps_to_control_reg_changes[time] = {
                        clean_signal_name: int_value
                    }
                else:
                    self.timestamps_to_control_reg_changes[time][clean_signal_name] = (
                        int_value
                    )

    """
    Must run after postprocess
    """

    def postprocess_control(self):
        control_group_events = {}  # cycle count --> [control groups that are active that cycle]
        control_reg_updates = {
            c: [] for c in self.cells_to_components
        }  # cell name --> (clock_cycle, updates)
        control_reg_per_cycle = {}  # clock cycle --> control_reg_update_type for leaf cell (longest cell name)
        # for now, control_reg_update_type will be one of "fsm", "par-done", "both"

        control_group_start_cycles = {}
        for ts in self.timestamps_to_control_group_events:
            if ts in self.timestamps_to_clock_cycles:
                clock_cycle = self.timestamps_to_clock_cycles[ts]
                events = self.timestamps_to_control_group_events[ts]
                for event in events:
                    group_name = event["group"]
                    if event["value"] == 1:  # control group started
                        control_group_start_cycles[group_name] = clock_cycle
                    elif event["value"] == 0:  # control group ended
                        for i in range(
                            control_group_start_cycles[group_name], clock_cycle
                        ):
                            if i in control_group_events:
                                control_group_events[i].add(group_name)
                            else:
                                control_group_events[i] = {group_name}

        for ts in self.timestamps_to_control_reg_changes:
            if ts in self.timestamps_to_clock_cycles:
                clock_cycle = self.timestamps_to_clock_cycles[ts]
                # control_reg_per_cycle[clock_cycle] = []
                events = self.timestamps_to_control_reg_changes[ts]
                cell_to_val_changes = {}
                cell_to_change_type = {}
                # we only care about registers when their write_enables are fired.
                for write_en in filter(
                    lambda e: e.endswith("write_en") and events[e] == 1, events.keys()
                ):
                    write_en_split = write_en.split(".")
                    reg_name = ".".join(write_en_split[:-1])
                    cell_name = ".".join(write_en_split[:-2])
                    in_signal = f"{reg_name}.in"
                    reg_new_value = events[in_signal] if in_signal in events else 0
                    if not (
                        reg_name in self.par_done_regs and reg_new_value == 0
                    ):  # ignore when pd values turn 0 since they are only useful when they are high
                        upd = f"{write_en_split[-2]}:{reg_new_value}"
                        if cell_name in cell_to_val_changes:
                            cell_to_val_changes[cell_name] += f", {upd}"
                        else:
                            cell_to_val_changes[cell_name] = upd
                        # update cell_to_change_type
                        if ".pd" in reg_name and cell_name not in cell_to_change_type:
                            cell_to_change_type[cell_name] = "par-done"
                        elif (
                            ".pd" in reg_name
                            and cell_to_change_type[cell_name] == "fsm"
                        ):
                            cell_to_change_type[cell_name] = "both"
                        elif (
                            ".fsm" in reg_name and cell_name not in cell_to_change_type
                        ):
                            cell_to_change_type[cell_name] = "fsm"
                        elif (
                            ".fsm" in reg_name
                            and cell_to_change_type[cell_name] == "par-done"
                        ):
                            cell_to_change_type[cell_name] = "both"
                        # m[cell_name].append((reg_name, reg_new_value, clock_cycle))
                for cell in cell_to_val_changes:
                    control_reg_updates[cell].append(
                        (clock_cycle, cell_to_val_changes[cell])
                    )
                if len(cell_to_change_type) > 0:
                    leaf_cell = sorted(
                        cell_to_change_type.keys(), key=(lambda k: k.count("."))
                    )[-1]
                    control_reg_per_cycle[clock_cycle] = cell_to_change_type[leaf_cell]
        return control_group_events, control_reg_updates, control_reg_per_cycle

    """
    Postprocess data mapping timestamps to events (signal changes)
    We have to postprocess instead of processing signals in a stream because
    signal changes that happen at the same time as a clock tick might be recorded
    *before* or *after* the clock change on the VCD file (hence why we can't process
    everything within a stream if we wanted to be precise)
    """

    def postprocess(self):
        clock_name = f"{self.main_component}.clk"
        clock_cycles = -1  # will be 0 on the 0th cycle
        started = False
        cell_active = set()
        group_active = set()
        structural_enable_active = set()
        cell_enable_active = set()
        primitive_enable = set()
        trace = {}
        trace_classified = []
        cell_to_active_cycles = {}  # cell --> [{"start": X, "end": Y, "length": Y - X}].

        # The events are "partial" because we don't know yet what the tid and pid would be.
        # (Will be filled in during create_timelines(); specifically in port_fsm_events())
        fsm_current = {fsm: 0 for fsm in self.fsms}  # fsm --> value

        probe_labels_to_sets = {
            "group_probe_out": group_active,
            "se_probe_out": structural_enable_active,
            "cell_probe_out": cell_enable_active,
            "primitive_probe_out": primitive_enable,
        }

        main_done = False  # Prevent creating a trace entry for the cycle where main.done is set high.
        for ts in self.timestamps_to_events:
            events = self.timestamps_to_events[ts]
            started = started or [
                x
                for x in events
                if x["signal"] == f"{self.main_component}.go" and x["value"] == 1
            ]
            if not started:  # only start counting when main component is on.
                continue
            # checking whether the timestamp has a rising edge
            if {"signal": clock_name, "value": 1} in events:
                clock_cycles += 1
                self.timestamps_to_clock_cycles[ts] = clock_cycles
            # Recording the data organization for every kind of probe so I don't forget. () is a set.
            # groups-active: cell --> (active groups)
            # cell-active: (cells)
            # structural-enable: cell --> { child --> (parents) }
            # cell-invoke: parent_cell --> { parent --> (cells) }
            # primitive-enable: cell --> { parent --> (primitives) }
            info_this_cycle = {
                "group-active": {},
                "cell-active": set(),
                "structural-enable": {},
                "cell-invoke": {},
                "primitive-enable": {},
            }
            for event in events:
                # check probe and cell signals to update currently active entities.
                signal_name = event["signal"]
                value = event["value"]
                if (
                    signal_name.endswith(".go") and value == 1
                ):  # cells have .go and .done
                    cell = signal_name.split(".go")[0]
                    cell_active.add(cell)
                    if cell not in cell_to_active_cycles:
                        cell_to_active_cycles[cell] = [{"start": clock_cycles}]
                    else:
                        cell_to_active_cycles[cell].append({"start": clock_cycles})
                if signal_name.endswith(".done") and value == 1:
                    cell = signal_name.split(".done")[0]
                    if (
                        cell == self.main_component
                    ):  # if main is done, we shouldn't compute a "trace" for this cycle. set flag to True.
                        main_done = True
                    cell_active.remove(cell)
                    current_segment = cell_to_active_cycles[cell][-1]
                    current_segment["end"] = clock_cycles
                    current_segment["length"] = clock_cycles - current_segment["start"]
                # process fsms
                if ".out[" in signal_name:
                    fsm_name = signal_name.split(".out[")[0]
                    cell_name = ".".join(fsm_name.split(".")[:-1])
                    if fsm_current[fsm_name] != value:
                        # record the (partial) end event of the previous value and begin event of the current value
                        partial_end_event = {
                            "name": str(fsm_current[fsm_name]),
                            "cat": "fsm",
                            "ph": "E",
                            "ts": clock_cycles * ts_multiplier,
                        }
                        partial_begin_event = {
                            "name": str(value),
                            "cat": "fsm",
                            "ph": "B",
                            "ts": clock_cycles * ts_multiplier,
                        }
                        self.partial_fsm_events[fsm_name].append(partial_end_event)
                        self.partial_fsm_events[fsm_name].append(partial_begin_event)
                        # update value
                        fsm_current[fsm_name] = value
                # process all probes.
                for probe_label in probe_labels_to_sets:
                    cutoff = f"_{probe_label}"
                    if cutoff in signal_name:
                        # record cell name instead of component name.
                        split = signal_name.split(cutoff)[0].split(DELIMITER)[:-1]
                        cell_name = ".".join(
                            signal_name.split(cutoff)[0].split(".")[:-1]
                        )
                        split.append(cell_name)
                        probe_info = tuple(split)
                        if value == 1:
                            probe_labels_to_sets[probe_label].add(probe_info)
                        elif value == 0:
                            probe_labels_to_sets[probe_label].remove(probe_info)
            if not main_done:
                # add all probe information
                info_this_cycle["cell-active"] = cell_active.copy()
                for group, cell_name in group_active:
                    if cell_name in info_this_cycle["group-active"]:
                        info_this_cycle["group-active"][cell_name].add(group)
                    else:
                        info_this_cycle["group-active"][cell_name] = {group}
                for child_group, parent_group, cell_name in structural_enable_active:
                    if cell_name not in info_this_cycle["structural-enable"]:
                        info_this_cycle["structural-enable"][cell_name] = {
                            child_group: {parent_group}
                        }
                    elif (
                        child_group
                        not in info_this_cycle["structural-enable"][cell_name]
                    ):
                        info_this_cycle["structural-enable"][cell_name][child_group] = {
                            parent_group
                        }
                    else:
                        info_this_cycle["structural-enable"][cell_name][
                            child_group
                        ].add(parent_group)
                for cell_name, parent_group, parent_cell_name in cell_enable_active:
                    if parent_cell_name not in info_this_cycle["cell-invoke"]:
                        info_this_cycle["cell-invoke"][parent_cell_name] = {
                            parent_group: {cell_name}
                        }
                    elif (
                        parent_group
                        not in info_this_cycle["cell-invoke"][parent_cell_name]
                    ):
                        info_this_cycle["cell-invoke"][parent_cell_name][
                            parent_group
                        ] = {cell_name}
                    else:
                        info_this_cycle["cell-invoke"][parent_cell_name][
                            parent_group
                        ].add(cell_name)
                for primitive_name, parent_group, cell_name in primitive_enable:
                    if cell_name not in info_this_cycle["primitive-enable"]:
                        info_this_cycle["primitive-enable"][cell_name] = {
                            parent_group: {primitive_name}
                        }
                    elif (
                        parent_group
                        not in info_this_cycle["primitive-enable"][cell_name]
                    ):
                        info_this_cycle["primitive-enable"][cell_name][parent_group] = {
                            primitive_name
                        }
                    else:
                        info_this_cycle["primitive-enable"][cell_name][
                            parent_group
                        ].add(primitive_name)
                stacks_this_cycle = create_cycle_trace(
                    info_this_cycle, self.cells_to_components, self.main_component, True
                )  # True to track primitives
                trace[clock_cycles] = stacks_this_cycle
                trace_classified.append(
                    classify_stacks(stacks_this_cycle, self.main_shortname)
                )
        self.clock_cycles = (
            clock_cycles  # last rising edge does not count as a full cycle (probably)
        )

        return trace, trace_classified, cell_to_active_cycles


def classify_stacks(stacks, main_shortname):
    # True if something "useful" is happening this cycle (group or primitive)
    acc = 0
    for stack in stacks:
        top = stack[-1]
        if "(primitive)" in top:
            acc += 1
        elif "[" not in top and top != main_shortname:  # group
            acc += 1

    return acc


"""
Generates a list of all of the components to potential cell names
`prefix` is the cell's "path" (ex. for a cell "my_cell" defined in "main", the prefix would be "TOP.toplevel.main")
The initial value of curr_component should be the top level/main component
"""


def build_components_to_cells(
    prefix, curr_component, cells_to_components, components_to_cells
):
    for cell, cell_component in cells_to_components[curr_component].items():
        if cell_component not in components_to_cells:
            components_to_cells[cell_component] = [f"{prefix}.{cell}"]
        else:
            components_to_cells[cell_component].append(f"{prefix}.{cell}")
        build_components_to_cells(
            prefix + f".{cell}",
            cell_component,
            cells_to_components,
            components_to_cells,
        )


"""
Reads json generated by component-cells backend to produce a mapping from all components
to cell names they could have.

NOTE: Cell names by this point don't contain the simulator-specific prefix. This will be
filled by VCDConverter.enddefinitions().
"""


def read_component_cell_names_json(json_file):
    cell_json = json.load(open(json_file))
    # For each component, contains a map from each cell name to its corresponding component
    # component name --> { cell name --> component name }
    cells_to_components = {}
    main_component = ""
    for curr_component_entry in cell_json:
        cell_map = {}  # mapping cell names to component names for all cells in the current component
        if curr_component_entry["is_main_component"]:
            main_component = curr_component_entry["component"]
        for cell_info in curr_component_entry["cell_info"]:
            cell_map[cell_info["cell_name"]] = cell_info["component_name"]
        cells_to_components[curr_component_entry["component"]] = cell_map
    components_to_cells = {
        main_component: [main_component]
    }  # come up with a better name for this
    build_components_to_cells(
        main_component, main_component, cells_to_components, components_to_cells
    )
    # semi-fully_qualified_cell_name --> component name (of cell)
    # I say semi-here because the prefix depends on the simulator + OS
    # (ex. "TOP.toplevel" for Verilator on ubuntu)
    cell_names_to_components = {}
    for component in components_to_cells:
        for cell in components_to_cells[component]:
            cell_names_to_components[cell] = component

    return main_component, cell_names_to_components, components_to_cells


"""
Creates a tree that encapsulates all stacks that occur within the program.
"""


def create_tree(timeline_map):
    node_id_acc = 0
    tree_dict = {}  # node id --> node name
    path_dict = {}  # stack list string --> list of node ids
    path_prefixes_dict = {}  # stack list string --> list of node ids
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
        for i in range(1, stack_len + 1):
            attempted_prefix = ";".join(stack[0 : stack_len - i])
            if attempted_prefix in path_prefixes_dict:
                prefix = attempted_prefix
                id_path_list = list(path_prefixes_dict[prefix])
                break
        # create nodes
        if prefix != "":
            new_nodes = stack[stack_len - i :]
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


def create_tree_rankings(
    trace, tree_dict, path_dict, path_to_edges, all_edges, dot_out_dir
):
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

    sorted_stack_list_items = sorted(
        stack_list_str_to_cycles.items(), key=(lambda item: len(item[1])), reverse=True
    )
    acc = 0
    rankings_out = open(os.path.join(dot_out_dir, "rankings.csv"), "w")
    rankings_out.write("Rank,#Cycles,Cycles-list\n")
    for stack_list_str, cycles in sorted_stack_list_items:
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
                    f.write(
                        f'\t{node} [label="{tree_dict[node]}",color="{INVISIBLE}",fontcolor="{INVISIBLE}"];\n'
                    )
            # write all edges.
            for edge in all_edges:
                if edge in stack_list_str_to_used_edges[stack_list_str]:
                    f.write(f"\t{edge} ; \n")
                else:
                    f.write(f'\t{edge} [color="{INVISIBLE}"]; \n')
            f.write("}")

        rankings_out.write(f"{acc},{len(cycles)},{';'.join(str(c) for c in cycles)}\n")


# one tree to summarize the entire execution.
def create_aggregate_tree(timeline_map, out_dir, tree_dict, path_dict):
    path_to_edges, all_edges = create_edge_dict(path_dict)

    leaf_nodes_dict = {
        node_id: 0 for node_id in tree_dict
    }  # how many times was this node a leaf?
    edges_dict = {}  # how many times was this edge active?

    for stack_list in timeline_map.values():
        edges_this_cycle = set()
        leaves_this_cycle = set()
        stacks_this_cycle = set(map(lambda stack: ";".join(stack), stack_list))
        for stack in stack_list:
            stack_id = ";".join(stack)
            for edge in path_to_edges[stack_id]:
                if edge not in edges_this_cycle:
                    if edge not in edges_dict:
                        edges_dict[edge] = 1
                    else:
                        edges_dict[edge] += 1
                    edges_this_cycle.add(edge)
            # record the leaf node. ignore all primitives as I think we care more about the group that called the primitive (up to debate)
            leaf_node = path_dict[stack_id][-1]
            if "primitive" in tree_dict[leaf_node]:
                leaf_node = path_dict[stack_id][-2]
                leaf_id = ";".join(stack[:-1])
                # if the current stack (minus primitive) is a prefix of another stack, then we shouldn't count it in as a leaf node.
                contained = False
                for other_stack in stacks_this_cycle:
                    if other_stack != stack_id and leaf_id in other_stack:
                        contained = True
                        break
                if contained:  # this is not actually a leaf node, so we should move onto the next leaf node.
                    continue
            if leaf_node not in leaves_this_cycle:
                leaf_nodes_dict[leaf_node] += 1
                leaves_this_cycle.add(leaf_node)

    # write the tree
    with open(os.path.join(out_dir, "aggregate.dot"), "w") as f:
        f.write("digraph aggregate {\n")
        # declare nodes
        for node in leaf_nodes_dict:
            if "primitive" in tree_dict[node]:
                f.write(
                    f'\t{node} [label="{tree_dict[node]}", style=filled, color="{ACTIVE_PRIMITIVE_COLOR}"];\n'
                )
            elif "[" in tree_dict[node] or "main" == tree_dict[node]:
                f.write(
                    f'\t{node} [label="{tree_dict[node]} ({leaf_nodes_dict[node]})", style=filled, color="{ACTIVE_CELL_COLOR}"];\n'
                )
            else:
                f.write(
                    f'\t{node} [label="{tree_dict[node]} ({leaf_nodes_dict[node]})", style=filled, color="{ACTIVE_GROUP_COLOR}"];\n'
                )
        # write edges with labels
        for edge in edges_dict:
            f.write(f'\t{edge} [label="{edges_dict[edge]}"]; \n')
        f.write("}")


def create_path_dot_str_dict(path_dict):
    path_to_dot_str = {}  # stack list string --> stack path representation on dot file.

    for path_id in path_dict:
        path = path_dict[path_id]
        path_acc = ""
        for node_id in path[0:-1]:
            path_acc += f"{node_id} -> "
        path_acc += f"{path[-1]}"
        path_to_dot_str[path_id] = path_acc

    return path_to_dot_str


def create_edge_dict(path_dict):
    path_to_edges = {}  # stack list string --> [edge string representation]
    all_edges = set()

    for path_id in path_dict:
        path = path_dict[path_id]
        edge_set = []
        for i in range(len(path) - 1):
            edge = f"{path[i]} -> {path[i + 1]}"
            edge_set.append(edge)
            all_edges.add(edge)
        path_to_edges[path_id] = edge_set

    return path_to_edges, list(sorted(all_edges))


def write_flame_map(flame_map, flame_out_file):
    with open(flame_out_file, "w") as flame_out:
        for stack in flame_map:
            flame_out.write(f"{stack} {flame_map[stack]}\n")


def write_flame_maps(
    flat_flame_map,
    scaled_flame_map,
    flames_out_dir,
    flame_out_file,
    scaled_flame_out_file=None,
):
    if not os.path.exists(flames_out_dir):
        os.mkdir(flames_out_dir)

    # write flat flame map
    write_flame_map(flat_flame_map, flame_out_file)

    # write scaled flame map
    if scaled_flame_out_file is None:
        scaled_flame_out_file = os.path.join(flames_out_dir, "scaled-flame.folded")
    write_flame_map(scaled_flame_map, scaled_flame_out_file)


"""
Creates flat and scaled flame maps from a trace.
"""


def create_flame_maps(trace):
    # flat flame graph; each par arm is counted for 1 cycle
    flat_flame_map = {}  # stack to number of cycles
    for i in trace:
        for stack_list in trace[i]:
            stack_id = ";".join(stack_list)
            if stack_id not in flat_flame_map:
                flat_flame_map[stack_id] = 1
            else:
                flat_flame_map[stack_id] += 1

    # scaled flame graph; each cycle is divided by the number of par arms that are concurrently active.
    scaled_flame_map = {}
    for i in trace:
        num_stacks = len(trace[i])
        cycle_slice = round(1 / num_stacks, 3)
        last_cycle_slice = 1 - (cycle_slice * (num_stacks - 1))
        acc = 0
        for stack_list in trace[i]:
            stack_id = ";".join(stack_list)
            slice_to_add = cycle_slice if acc < num_stacks - 1 else last_cycle_slice
            if stack_id not in scaled_flame_map:
                scaled_flame_map[stack_id] = slice_to_add * SCALED_FLAME_MULTIPLIER
            else:
                scaled_flame_map[stack_id] += slice_to_add * SCALED_FLAME_MULTIPLIER
            acc += 1

    return flat_flame_map, scaled_flame_map


def create_slideshow_dot(timeline_map, dot_out_dir, flame_out_file, flames_out_dir):
    if not os.path.exists(dot_out_dir):
        os.mkdir(dot_out_dir)

    # only produce trees for every cycle if we don't exceed TREE_PICTURE_LIMIT
    if len(timeline_map) > TREE_PICTURE_LIMIT:
        print(
            f"Simulation exceeds {TREE_PICTURE_LIMIT} cycles, skipping slideshow trees for every cycle..."
        )
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
                    f.write(
                        f'\t{node} [label="{tree_dict[node]}",color="{INVISIBLE}",fontcolor="{INVISIBLE}"];\n'
                    )
            # write all edges.
            for edge in all_edges:
                if edge in used_edges.keys():
                    f.write(f"\t{edge} ; \n")
                else:
                    f.write(f'\t{edge} [color="{INVISIBLE}"]; \n')
            f.write("}")


def dump_trace(trace, out_dir):
    with open(os.path.join(out_dir, "trace.json"), "w") as json_out:
        json.dump(trace, json_out, indent=2)


class TimelineCell:
    # bookkeeping for forming cells and their groups
    def __init__(self, name, pid):
        self.name = name
        self.pid = pid
        self.tid = 1  # the cell itself gets tid 1, FSMs gets 2+, followed by parallel executions of groups
        self.tid_acc = 2
        self.fsm_to_tid = {}  # contents: group/fsm --> tid
        self.currently_active_group_to_tid = {}
        self.queued_tids = []

    def get_fsm_pid_tid(self, fsm_name):
        if fsm_name not in self.fsm_to_tid:
            self.fsm_to_tid[fsm_name] = self.tid_acc
            self.tid_acc += 1
        return (self.pid, self.fsm_to_tid[fsm_name])

    def get_group_pid_tid(self, group_name):
        return (self.pid, self.currently_active_group_to_tid[group_name])

    def add_group(self, group_name):
        if (
            group_name in self.currently_active_group_to_tid
        ):  # no-op since the group is already registered.
            return self.currently_active_group_to_tid[group_name]
        if len(self.queued_tids) > 0:
            group_tid = min(self.queued_tids)
            self.queued_tids.remove(group_tid)
        else:
            group_tid = self.tid_acc
            self.tid_acc += 1
        self.currently_active_group_to_tid[group_name] = group_tid
        return (self.pid, group_tid)

    def remove_group(self, group_name):
        group_tid = self.currently_active_group_to_tid[group_name]
        self.queued_tids.append(group_tid)
        del self.currently_active_group_to_tid[group_name]
        return (self.pid, group_tid)


def write_timeline_event(event, out_file):
    global num_timeline_events
    if num_timeline_events == 0:  # shouldn't prepend a comma on the first entry
        out_file.write(f"\n{JSON_INDENT}{json.dumps(event)}")
    else:
        out_file.write(f",\n{JSON_INDENT}{json.dumps(event)}")
    num_timeline_events += 1


def port_fsm_and_control_events(
    partial_fsm_events, control_updates, cell_to_info, cell_name, out_file
):
    for fsm_name in list(partial_fsm_events.keys()):
        # NOTE: uncomment below to bring back FSM tracks to the timeline.
        # fsm_cell_name = ".".join(fsm_name.split(".")[:-1])
        # if fsm_cell_name == cell_name:
        #     (fsm_pid, fsm_tid) = cell_to_info[cell_name].get_fsm_pid_tid(fsm_name)
        #     for entry in partial_fsm_events[fsm_name]:
        #         entry["pid"] = fsm_pid
        #         entry["tid"] = fsm_tid
        #         write_timeline_event(entry, out_file)
        del partial_fsm_events[fsm_name]
    for cycle, update in control_updates[cell_name]:
        # FIXME: rename the function
        (control_pid, control_tid) = cell_to_info[cell_name].get_fsm_pid_tid("CTRL")
        begin_event = {
            "name": update,
            "cat": "CTRL",
            "ph": "B",
            "ts": cycle * ts_multiplier,
            "pid": control_pid,
            "tid": control_tid,
        }
        end_event = {
            "name": update,
            "cat": "CTRL",
            "ph": "E",
            "ts": (cycle + 1) * ts_multiplier,
            "pid": control_pid,
            "tid": control_tid,
        }
        write_timeline_event(begin_event, out_file)
        write_timeline_event(end_event, out_file)
    del control_updates[cell_name]


def compute_timeline(
    trace, partial_fsm_events, control_updates, main_component, out_dir
):
    # generate the JSON on the fly instead of storing everything in a list to save memory
    out_path = os.path.join(out_dir, "timeline-dump.json")
    out_file = open(out_path, "w", encoding="utf-8")
    # start the JSON file
    out_file.write(f'{{\n{JSON_INDENT}"traceEvents": [')
    # each cell gets its own pid. The cell's lifetime is tid 1, followed by the FSM(s), then groups
    # main component gets pid 1
    cell_to_info = {main_component: TimelineCell(main_component, 1)}
    # generate JSON for all FSM events in main
    port_fsm_and_control_events(
        partial_fsm_events, control_updates, cell_to_info, main_component, out_file
    )
    group_to_parent_cell = {}
    pid_acc = 2
    currently_active = set()
    main_name = main_component.split(".")[-1]
    for i in trace:
        active_this_cycle = set()
        for stack in trace[i]:
            stack_acc = main_component
            current_cell = main_component  # need to keep track of cells in case we have a structural group enable.
            for stack_elem in stack:
                name = None
                if " [" in stack_elem:  # cell
                    stack_acc += "." + stack_elem.split(" [")[0]
                    name = stack_acc
                    current_cell = name
                    if name not in cell_to_info:  # cell is not registered yet
                        cell_to_info[name] = TimelineCell(name, pid_acc)
                        # generate JSON for all FSM events in this cell
                        port_fsm_and_control_events(
                            partial_fsm_events,
                            control_updates,
                            cell_to_info,
                            name,
                            out_file,
                        )
                        pid_acc += 1
                elif "(primitive)" in stack_elem:  # ignore primitives for now.
                    continue
                elif (
                    stack_elem == main_name
                ):  # don't accumulate to the stack if your name is main.
                    stack_acc = stack_acc
                    name = main_component
                else:  # group
                    name = stack_acc + "." + stack_elem
                    group_to_parent_cell[name] = current_cell
                active_this_cycle.add(name)
        for nonactive_element in currently_active.difference(
            active_this_cycle
        ):  # element that was previously active but no longer is.
            # make end event
            end_event = create_timeline_event(
                nonactive_element, i, "E", cell_to_info, group_to_parent_cell
            )
            write_timeline_event(end_event, out_file)
        for newly_active_element in active_this_cycle.difference(
            currently_active
        ):  # element that started to be active this cycle.
            begin_event = create_timeline_event(
                newly_active_element, i, "B", cell_to_info, group_to_parent_cell
            )
            write_timeline_event(begin_event, out_file)
        currently_active = active_this_cycle

    for still_active_element in (
        currently_active
    ):  # need to close any elements that are still active at the end of the simulation
        end_event = create_timeline_event(
            still_active_element, len(trace), "E", cell_to_info, group_to_parent_cell
        )
        write_timeline_event(end_event, out_file)

    # close off the json
    out_file.write("\t\t]\n}")
    out_file.close()


"""
Creates a JSON entry for traceEvents.
element_name: fully qualified name of cell/group
cycle: timestamp of the event, in cycles
event_type: "B" for begin event, "E" for end event
"""


def create_timeline_event(
    element_name, cycle, event_type, cell_to_info, group_to_parent_cell
):
    if element_name in cell_to_info:  # cell
        event = {
            "name": element_name,
            "cat": "cell",
            "ph": event_type,
            "pid": cell_to_info[element_name].pid,
            "tid": 1,
            "ts": cycle * ts_multiplier,
        }
    else:  # group; need to extract the cell name to obtain tid and pid.
        cell_name = group_to_parent_cell[element_name]
        cell_info = cell_to_info[cell_name]
        if event_type == "B":
            (pid, tid) = cell_info.add_group(element_name)
        else:
            (pid, tid) = cell_info.remove_group(element_name)
        event = {
            "name": element_name.split(".")[
                -1
            ],  # take only the group name for easier visibility
            "cat": "group",
            "ph": event_type,
            "pid": pid,
            "tid": tid,
            "ts": cycle * ts_multiplier,
        }
    return event


def write_cell_stats(
    cell_to_active_cycles,
    cats_to_cycles,
    cells_to_components,
    component_to_num_fsms,
    total_cycles,
    out_dir,
):
    fieldnames = [
        "cell-name",
        "num-fsms",
        "useful-cycles",
        "total-cycles",
        "times-active",
        "avg",
    ] + [f"{cat} (%)" for cat in cats_to_cycles]  # fields in CSV file
    stats = []
    totals = {fieldname: 0 for fieldname in fieldnames}
    for cell in cell_to_active_cycles:
        component = cells_to_components[cell]
        num_fsms = component_to_num_fsms[component]
        cell_total_cycles = 0
        times_active = len(cell_to_active_cycles[cell])
        cell_cat = {cat: set() for cat in cats_to_cycles}
        for elem in cell_to_active_cycles[cell]:
            cell_total_cycles += elem["length"]
            active_cycle_list = set(range(elem["start"], elem["end"]))
            for cat in cats_to_cycles:
                cell_cat[cat].update(
                    active_cycle_list.intersection(cats_to_cycles[cat])
                )

        avg_cycles = round(cell_total_cycles / times_active, 2)
        stats_dict = {
            "cell-name": f"{cell} [{component}]",
            "num-fsms": num_fsms,
            "useful-cycles": len(cell_cat["group/primitive"]) + len(cell_cat["other"]),
            "total-cycles": cell_total_cycles,
            "times-active": times_active,
            "avg": avg_cycles,
        }
        # aggregate stats that should be summed over
        totals["num-fsms"] += num_fsms
        for cat in cats_to_cycles:
            stats_dict[f"{cat} (%)"] = round(
                (len(cell_cat[cat]) / cell_total_cycles) * 100, 1
            )
        stats.append(stats_dict)
    # total: aggregate other stats that shouldn't just be summed over
    totals["cell-name"] = "TOTAL"
    totals["total-cycles"] = total_cycles
    for cat in cats_to_cycles:
        if cat == "group/primitive" or cat == "other":
            totals["useful-cycles"] += len(cats_to_cycles[cat])
        totals[f"{cat} (%)"] = round((len(cats_to_cycles[cat]) / total_cycles) * 100, 1)
    totals["avg"] = "-"
    stats.sort(key=lambda e: e["total-cycles"], reverse=True)
    stats.append(totals)  # total should come at the end
    with open(os.path.join(out_dir, "cell-stats.csv"), "w") as csvFile:
        writer = csv.DictWriter(csvFile, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(stats)


class SourceLoc:
    def __init__(self, json_dict):
        self.filename = os.path.basename(json_dict["filename"])
        self.linenum = json_dict["linenum"]
        self.varname = json_dict["varname"]

    def __repr__(self):
        return f"{self.filename}: {self.linenum}"


def read_adl_mapping_file(adl_mapping_file):
    component_mappings = {}  # component --> (filename, linenum)
    cell_mappings = {}  # component --> {cell --> (filename, linenum)}
    group_mappings = {}  # component --> {group --> (filename, linenum)}
    with open(adl_mapping_file, "r") as json_file:
        json_data = json.load(json_file)
    for component_dict in json_data:
        component_name = component_dict["component"]
        component_mappings[component_name] = SourceLoc(component_dict)
        cell_mappings[component_name] = {}
        for cell_dict in component_dict["cells"]:
            cell_mappings[component_name][cell_dict["name"]] = SourceLoc(cell_dict)
        # probably worth removing code clone at some point
        group_mappings[component_name] = {}
        for group_dict in component_dict["groups"]:
            group_mappings[component_name][group_dict["name"]] = SourceLoc(group_dict)
    return component_mappings, cell_mappings, group_mappings


"""
Creates ADL and Mixed (ADL + Calyx) versions of flame graph maps.
"""


def convert_flame_map(flame_map, adl_mapping_file):
    component_map, cell_map, group_map = read_adl_mapping_file(adl_mapping_file)
    adl_flame_map = {}
    mixed_flame_map = {}

    for stack in sorted(flame_map.keys()):
        cycles = flame_map[stack]
        adl_stack = []
        mixed_stack = []
        curr_component = None
        for stack_elem in stack.split(";"):
            # going to start by assuming "main" is the entrypoint.
            if stack_elem == "main":
                curr_component = stack_elem
                sourceloc = component_map[stack_elem]
                mixed_stack_elem = f"main {{{sourceloc}}}"
                adl_stack_elem = mixed_stack_elem
            elif "[" in stack_elem:  # invocation of component cell
                cell = stack_elem.split("[")[0].strip()
                cell_sourceloc = cell_map[curr_component][cell]
                cell_component = stack_elem.split("[")[1].split("]")[0]
                cell_component_sourceloc = component_map[cell_component]
                mixed_stack_elem = f"{cell} {{{cell_sourceloc}}} [{cell_component} {{{cell_component_sourceloc}}}]"
                adl_stack_elem = f"{cell_sourceloc.varname} {{{cell_sourceloc}}} [{cell_component_sourceloc.varname} {{{cell_component_sourceloc}}}]"
                curr_component = cell_component
            elif "(primitive)" in stack_elem:  # primitive
                primitive = stack_elem.split("(primitive)")[0].strip()
                primitive_sourceloc = cell_map[curr_component][primitive]
                mixed_stack_elem = f"{stack_elem} {{{primitive_sourceloc}}}"
                adl_stack_elem = (
                    f"{primitive_sourceloc.varname} {{{primitive_sourceloc}}}"
                )
            else:  # group
                # ignore compiler-generated groups (invokes) for now...
                if stack_elem in group_map[curr_component]:
                    sourceloc = group_map[curr_component][stack_elem]
                    adl_stack_elem = f"{sourceloc.varname} {{{sourceloc}}}"
                else:
                    sourceloc = "compiler-generated"
                    adl_stack_elem = sourceloc
                mixed_stack_elem = f"{stack_elem} {{{sourceloc}}}"
            adl_stack.append(adl_stack_elem)
            mixed_stack.append(mixed_stack_elem)
        # multiple Calyx stacks might have the same ADL stack (same source). If the ADL/mixed stack already exists in the map, we add the cycles from this Calyx stack.
        adl_stack_str = ";".join(adl_stack)
        mixed_stack_str = ";".join(mixed_stack)
        if adl_stack_str in adl_flame_map:
            adl_flame_map[adl_stack_str] += cycles
        else:
            adl_flame_map[adl_stack_str] = cycles
        if mixed_stack_str in mixed_flame_map:
            mixed_flame_map[mixed_stack_str] += cycles
        else:
            mixed_flame_map[mixed_stack_str] = cycles

    return adl_flame_map, mixed_flame_map


"""
# Returns { cell --> fsm fully qualified names }
Returns a set of all fsms with fully qualified fsm names
"""


def read_tdcc_file(fsm_json_file, components_to_cells):
    json_data = json.load(open(fsm_json_file))
    # cell_to_fsms = {} # cell --> fully qualified fsm names
    fully_qualified_fsms = set()
    par_info = {}  # fully qualified par name --> [fully-qualified par children name]
    reverse_par_info = {}  # fully qualified par name --> [fully-qualified par parent name]
    cell_to_pars = {}
    cell_to_groups_to_par_parent = {}  # cell --> { group --> name of par parent group}. Kind of like reverse_par_info but for normal groups
    # this is necessary because if a nested par occurs simultaneously with a group, we don't want the nested par to be a parent of the group
    par_done_regs = set()
    component_to_fsm_acc = {component: 0 for component in components_to_cells}
    for json_entry in json_data:
        if "Fsm" in json_entry:
            entry = json_entry["Fsm"]
            fsm_name = entry["fsm"]
            component = entry["component"]
            component_to_fsm_acc[component] += 1
            for cell in components_to_cells[component]:
                fully_qualified_fsm = ".".join((cell, fsm_name))
                fully_qualified_fsms.add(fully_qualified_fsm)
                # if cell not in cell_to_fsms:
                #     cell_to_fsms[cell] = [fully_qualified_fsm]
                # else:
                #     cell_to_fsms[cell].append(fully_qualified_fsm)
        if "Par" in json_entry:
            entry = json_entry["Par"]
            par = entry["par_group"]
            component = entry["component"]
            child_par_groups = []
            for cell in components_to_cells[component]:
                fully_qualified_par = ".".join((cell, par))
                if cell in cell_to_pars:
                    cell_to_pars[cell].add(fully_qualified_par)
                else:
                    cell_to_pars[cell] = {fully_qualified_par}
                for child in entry["child_groups"]:
                    child_name = child["group"]
                    if child_name.startswith(
                        "par"
                    ):  # FIXME: heuristic. might be best to filter later with list of pars
                        fully_qualified_child_name = ".".join((cell, child_name))
                        child_par_groups.append(fully_qualified_child_name)
                        if fully_qualified_child_name in reverse_par_info:
                            reverse_par_info[fully_qualified_child_name].append(
                                fully_qualified_par
                            )
                        else:
                            reverse_par_info[fully_qualified_child_name] = [
                                fully_qualified_par
                            ]
                    else:  # normal group
                        if cell in cell_to_groups_to_par_parent:
                            if child_name in cell_to_groups_to_par_parent[cell]:
                                cell_to_groups_to_par_parent[cell][child_name].add(par)
                            else:
                                cell_to_groups_to_par_parent[cell][child_name] = {par}
                        else:
                            cell_to_groups_to_par_parent[cell] = {child_name: {par}}
                    # register
                    child_pd_reg = child["register"]
                    par_done_regs.add(".".join((cell, child_pd_reg)))
                par_info[fully_qualified_par] = child_par_groups

    return (
        fully_qualified_fsms,
        component_to_fsm_acc,
        par_info,
        reverse_par_info,
        cell_to_pars,
        par_done_regs,
        cell_to_groups_to_par_parent,
    )


"""
Give a partial ordering for pars
(1) order based on cells
(2) for pars in the same cell, order based on dependencies information
"""


def order_pars(cell_to_pars, par_deps, rev_par_deps, signal_prefix):
    ordered = {}  # cell --> ordered par names
    for cell in sorted(cell_to_pars, key=(lambda c: c.count("."))):
        ordered[cell] = []
        pars = cell_to_pars[cell]
        # start with pars with no parent
        worklist = list(pars.difference(rev_par_deps))
        while len(worklist) > 0:
            par = worklist.pop(0)
            if par not in ordered:
                ordered[cell].append(par)  # f"{signal_prefix}.{par}"
            # get all the children of this par
            worklist += par_deps[par]
    return ordered


def add_par_to_trace(
    trace,
    par_trace,
    cells_to_ordered_pars,
    cell_to_groups_to_par_parent,
    main_shortname,
):
    new_trace = {i: [] for i in trace}
    for i in trace:
        if i in par_trace:
            for events_stack in trace[i]:
                new_events_stack = []
                for construct in events_stack:
                    new_events_stack.append(construct)
                    if construct == main_shortname:  # main
                        current_cell = main_shortname
                    elif " [" in construct:  # cell detected
                        current_cell += "." + construct.split(" [")[0]
                    elif "(primitive)" not in construct:  # group
                        # handling the edge case of nested pars concurrent with groups; pop any pars that aren't this group's parent.
                        # soooooooo ugly
                        if (
                            current_cell in cell_to_groups_to_par_parent
                            and construct in cell_to_groups_to_par_parent[current_cell]
                        ):
                            group_parents = cell_to_groups_to_par_parent[current_cell][
                                construct
                            ]
                            parent_found = False
                            while (
                                len(new_events_stack) > 2
                                and "(ctrl)" in new_events_stack[-2]
                            ):  # FIXME: this hack only works because par is the only ctrl element rn...
                                for parent in group_parents:
                                    if f"{parent} (ctrl)" == new_events_stack[-2]:
                                        parent_found = True
                                        break
                                if parent_found:
                                    break
                                new_events_stack.pop(-2)
                        continue
                    else:
                        continue
                    # get all of the active pars from this cell
                    if current_cell in cells_to_ordered_pars:
                        active_from_cell = par_trace[i].intersection(
                            cells_to_ordered_pars[current_cell]
                        )
                        for par_group_active in sorted(
                            active_from_cell,
                            key=(
                                lambda p: cells_to_ordered_pars[current_cell].index(p)
                            ),
                        ):
                            par_group_name = par_group_active.split(".")[-1] + " (ctrl)"
                            new_events_stack.append(par_group_name)
                new_trace[i].append(new_events_stack)
        else:
            new_trace[i] = trace[i].copy()

    return new_trace


def create_simple_flame_graph(classified_trace, control_reg_updates, out_dir):
    flame_base_map = {
        "group/primitive": [],
        "fsm": [],
        "par-done": [],
        "mult-ctrl": [],  # fsm and par-done. Have not seen this yet
        "other": [],
    }
    for i in range(len(classified_trace)):
        if classified_trace[i] > 0:
            flame_base_map["group/primitive"].append(i)
        elif (
            i not in control_reg_updates
        ):  # I suspect this is 1 cycle to execute a combinational group.
            flame_base_map["other"].append(i)
            classified_trace[i] = 1  # FIXME: hack to flag this as a "useful" cycle
        elif control_reg_updates[i] == "both":
            flame_base_map["fsm + par-done"].append(i)
        else:
            flame_base_map[control_reg_updates[i]].append(i)
    # modify names to contain their cycles (for easier viewing)
    flame_map = {key: len(flame_base_map[key]) for key in flame_base_map}
    for label in list(flame_map.keys()):
        flame_map[f"{label} ({flame_map[label]})"] = flame_map[label]
        del flame_map[label]
    write_flame_map(flame_map, os.path.join(out_dir, "overview.folded"))
    return flame_base_map


def main(
    vcd_filename, cells_json_file, tdcc_json_file, adl_mapping_file, out_dir, flame_out
):
    print(f"Start time: {datetime.now()}")
    main_shortname, cells_to_components, components_to_cells = (
        read_component_cell_names_json(cells_json_file)
    )
    (
        fully_qualified_fsms,
        component_to_num_fsms,
        par_dep_info,
        reverse_par_dep_info,
        cell_to_pars,
        par_done_regs,
        cell_to_groups_to_par_parent,
    ) = read_tdcc_file(tdcc_json_file, components_to_cells)
    # moving output info out of the converter
    fsm_events = {
        fsm: [{"name": str(0), "cat": "fsm", "ph": "B", "ts": 0}]
        for fsm in fully_qualified_fsms
    }  # won't be fully filled in until create_timeline()
    print(f"Start reading VCD: {datetime.now()}")
    converter = VCDConverter(
        main_shortname,
        cells_to_components,
        fully_qualified_fsms,
        fsm_events,
        set(par_dep_info.keys()),
        par_done_regs,
    )
    vcdvcd.VCDVCD(vcd_filename, callbacks=converter)
    signal_prefix = converter.signal_prefix
    main_fullname = converter.main_component
    print(f"Start Postprocessing VCD: {datetime.now()}")
    
    trace, trace_classified, cell_to_active_cycles = (
        converter.postprocess()
    )  # trace contents: cycle # --> list of stacks, trace_classified is a list: cycle # (indices) --> # useful stacks
    control_groups_trace, control_reg_updates, control_reg_updates_per_cycle = (
        converter.postprocess_control()
    )
    cell_to_ordered_pars = order_pars(
        cell_to_pars, par_dep_info, reverse_par_dep_info, signal_prefix
    )
    trace_with_pars = add_par_to_trace(
        trace,
        control_groups_trace,
        cell_to_ordered_pars,
        cell_to_groups_to_par_parent,
        main_shortname,
    )
    print(f"End Postprocessing VCD: {datetime.now()}")
    print(f"End reading VCD: {datetime.now()}")
    del converter

    if len(trace) < 100:
        for i in trace_with_pars:
            print(i)
            for stack in trace_with_pars[i]:
                print(f"\t{stack}")

    if not os.path.exists(out_dir):
        os.mkdir(out_dir)
    cats_to_cycles = create_simple_flame_graph(
        trace_classified, control_reg_updates_per_cycle, out_dir
    )
    print(f"End creating simple flame graph: {datetime.now()}")
    write_cell_stats(
        cell_to_active_cycles,
        cats_to_cycles,
        cells_to_components,
        component_to_num_fsms,
        len(trace),
        out_dir,
    )
    print(f"End writing cell stats: {datetime.now()}")
    tree_dict, path_dict = create_tree(trace)
    path_to_edges, all_edges = create_edge_dict(path_dict)

    create_aggregate_tree(trace, out_dir, tree_dict, path_dict)
    create_tree_rankings(trace, tree_dict, path_dict, path_to_edges, all_edges, out_dir)
    flat_flame_map, scaled_flame_map = create_flame_maps(trace_with_pars)
    write_flame_maps(flat_flame_map, scaled_flame_map, out_dir, flame_out)

    compute_timeline(trace, fsm_events, control_reg_updates, main_fullname, out_dir)

    if adl_mapping_file is not None:  # emit ADL flame graphs.
        print("Computing ADL flames...")
        adl_flat_flame, mixed_flat_flame = convert_flame_map(
            flat_flame_map, adl_mapping_file
        )
        adl_scaled_flame, mixed_scaled_flame = convert_flame_map(
            scaled_flame_map, adl_mapping_file
        )
        adl_flat_flame_file = os.path.join(out_dir, "adl-flat-flame.folded")
        adl_scaled_flame_file = os.path.join(out_dir, "adl-scaled-flame.folded")
        write_flame_maps(
            adl_flat_flame,
            adl_scaled_flame,
            out_dir,
            adl_flat_flame_file,
            adl_scaled_flame_file,
        )

        mixed_flat_flame_file = os.path.join(out_dir, "mixed-flat-flame.folded")
        mixed_scaled_flame_file = os.path.join(out_dir, "mixed-scaled-flame.folded")
        write_flame_maps(
            mixed_flat_flame,
            mixed_scaled_flame,
            out_dir,
            mixed_flat_flame_file,
            mixed_scaled_flame_file,
        )

    print(f"End time: {datetime.now()}")


if __name__ == "__main__":
    if len(sys.argv) > 5:
        vcd_filename = sys.argv[1]
        cells_json = sys.argv[2]
        fsms_json = sys.argv[3]
        out_dir = sys.argv[4]
        flame_out = sys.argv[5]
        if len(sys.argv) > 6:
            adl_mapping_file = sys.argv[6]
        else:
            adl_mapping_file = None
        print(f"ADL mapping file: {adl_mapping_file}")
        main(vcd_filename, cells_json, fsms_json, adl_mapping_file, out_dir, flame_out)
    else:
        args_desc = [
            "VCD_FILE",
            "CELLS_JSON",
            "FSMS_JSON",
            "OUT_DIR",
            "FLATTENED_FLAME_OUT",
            "[ADL_MAP_JSON]",
        ]
        print(f"Usage: {sys.argv[0]} {' '.join(args_desc)}")
        print("CELLS_JSON: Run the `component_cells` tool")
        print("CELLS_FOR_TIMELINE is an optional ")
        sys.exit(-1)
