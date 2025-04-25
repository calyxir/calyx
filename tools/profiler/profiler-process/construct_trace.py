import vcdvcd

from errors import ProfilerException
from visuals.timeline import ts_multiplier

DELIMITER = "___"


def remove_size_from_name(name: str) -> str:
    """changes e.g. "state[2:0]" to "state" """
    return name.split("[")[0]


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


def create_cycle_trace(
    info_this_cycle,
    cells_to_components,
    shared_cell_map,
    main_component,
    include_primitives,
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
                # TODO: if rewritten... then look for the rewritten cell from cell-active
                # probably worth putting some info in the flame graph that the cell is rewritten from the originally coded one?
                current_component = (
                    cells_to_components[current_cell]
                    if current_cell != main_component
                    else "main"
                )
                cell_split = invoked_cell.split(".")
                cell_shortname = cell_split[-1]
                cell_prefix = ".".join(cell_split[:-1])
                if cell_shortname in shared_cell_map[current_component]:
                    replacement_cell_shortname = shared_cell_map[current_component][
                        cell_shortname
                    ]
                    replacement_cell = f"{cell_prefix}.{replacement_cell_shortname}"
                    if replacement_cell not in info_this_cycle["cell-active"]:
                        raise ProfilerException(
                            f"Replacement cell {replacement_cell_shortname} for cell {invoked_cell} not found in active cells list!\n{info_this_cycle['cell-active']}"
                        )
                    cell_worklist.append(replacement_cell)
                    cell_component = cells_to_components[replacement_cell]
                    parent = f"{current_cell}.{cell_invoker_group}"
                    i_mapping[replacement_cell] = i_mapping[parent] + [
                        f"{cell_shortname} ({replacement_cell_shortname}) [{cell_component}]"
                    ]
                    parents.add(parent)
                elif invoked_cell in info_this_cycle["cell-active"]:
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
        """
        Decide which signals we need to extract value change information from.
        """
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
            raise ProfilerException(f"Found multiple clocks: {clock_filter} Exiting...")
        elif len(clock_filter) == 0:
            raise ProfilerException("Can't find the clock? Exiting...")
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
        # separating probe + cell signals from TDCC/control register signals so we can have a
        # control-signal-free version of the trace.
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
        """
        Reading through value changes and preserving timestamps to value changes for
        all signals we "care about".
        """
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

    def postprocess(self, shared_cells_map):
        """
        Postprocess data mapping timestamps to events (signal changes)
        We have to postprocess instead of processing signals in a stream because
        signal changes that happen at the same time as a clock tick might be recorded
        *before* or *after* the clock change on the VCD file (hence why we can't process
        everything within a stream if we wanted to be precise)
        """

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
        cell_to_active_cycles_summary = {}  # cell --> {"num-times-active": _, "active-cycles": []}
        # we lose information about the length of each segment but we can retrieve that information from the timeline

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
                    if cell not in cell_to_active_cycles_summary:
                        cell_to_active_cycles_summary[cell] = {
                            "num-times-active": 1,
                            "active-cycles": set(),
                        }  # add active-cycles when accounting for cell_active at the end
                    else:
                        cell_to_active_cycles_summary[cell]["num-times-active"] += 1
                if signal_name.endswith(".done") and value == 1:
                    cell = signal_name.split(".done")[0]
                    if (
                        cell == self.main_component
                    ):  # if main is done, we shouldn't compute a "trace" for this cycle. set flag to True.
                        main_done = True
                    cell_active.remove(cell)
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
                # accumulate cycles active for each cell that was active
                for cell in cell_active:
                    cell_to_active_cycles_summary[cell]["active-cycles"].add(
                        clock_cycles
                    )
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
                    info_this_cycle,
                    self.cells_to_components,
                    shared_cells_map,
                    self.main_component,
                    True,
                )  # True to track primitives
                trace[clock_cycles] = stacks_this_cycle
                trace_classified.append(
                    classify_stacks(stacks_this_cycle, self.main_shortname)
                )
        self.clock_cycles = (
            clock_cycles  # last rising edge does not count as a full cycle (probably)
        )

        return trace, trace_classified, cell_to_active_cycles_summary

    def postprocess_control(self):
        """
        Collects information on control register (fsm, pd) updates.
        Must run after self.postprocess() because this function relies on self.timestamps_to_clock_cycles
        (which gets filled in during self.postprocess()).
        """
        control_group_events = {}  # cycle count --> [control groups that are active that cycle]
        control_reg_updates = {
            c: [] for c in self.cells_to_components
        }  # cell name --> (clock_cycle, updates)
        control_reg_per_cycle = {}  # clock cycle --> control_reg_update_type for leaf cell (longest cell name)
        # for now, control_reg_update_type will be one of "fsm", "par-done", "both"

        control_group_start_cycles = {}
        control_group_to_summary = {}  # group --> {"num-times-active": _, "active-cycles": []}. Used in
        for ts in self.timestamps_to_control_group_events:
            if ts in self.timestamps_to_clock_cycles:
                clock_cycle = self.timestamps_to_clock_cycles[ts]
                events = self.timestamps_to_control_group_events[ts]
                for event in events:
                    group_name = event["group"]
                    if group_name not in control_group_to_summary:
                        control_group_to_summary[group_name] = {
                            "num-times-active": 0,
                            "active-cycles": [],
                        }
                    if event["value"] == 1:  # control group started
                        control_group_start_cycles[group_name] = clock_cycle
                        control_group_to_summary[group_name]["num-times-active"] += 1
                    elif event["value"] == 0:  # control group ended
                        active_range = range(
                            control_group_start_cycles[group_name], clock_cycle
                        )
                        control_group_to_summary[group_name]["active-cycles"] += list(
                            active_range
                        )
                        for i in active_range:
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
        return (
            control_group_events,
            control_group_to_summary,
            control_reg_updates,
            control_reg_per_cycle,
        )


def order_pars(cell_to_pars, par_deps, rev_par_deps, signal_prefix):
    """
    Give a partial ordering for pars so we know when multiple pars occur simultaneously, what order
    we should add them to the trace.
    (1) order based on cells
    (2) for pars in the same cell, order based on dependencies information
    """
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
    """
    Adds par groups (created by TDCC) to an existing trace.
    """
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
                            ):  # NOTE: fix in future when there are multiple "ctrl" elements
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
