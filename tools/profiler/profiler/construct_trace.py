import vcdvcd

from profiler.classes import (
    CellMetadata,
    ControlMetadata,
    CycleTrace,
    Utilization,
    UtilizationCycleTrace,
    TraceData,
    StackElement,
    StackElementType,
    ControlRegUpdateType,
)
from dataclasses import dataclass
from collections import defaultdict
from profiler.errors import ProfilerException

DELIMITER = "___"


def remove_size_from_name(name: str) -> str:
    """changes e.g. "state[2:0]" to "state" """
    return name.split("[")[0]


@dataclass(frozen=True)
class WaveformEvent:
    signal: str
    value: int

    def __repr__(self):
        return f"({self.signal}, {self.value})"


class VCDConverter(vcdvcd.StreamParserCallbacks):
    def __init__(self, cell_metadata, control_metadata, tracedata):
        super().__init__()
        self.cell_metadata: CellMetadata = cell_metadata
        self.control_metadata: ControlMetadata = control_metadata
        self.tracedata: TraceData = tracedata
        self.timestamps_to_events: dict[int, list[WaveformEvent]] = {}  # timestamps to
        self.timestamps_to_clock_cycles: dict[int, int] = {}
        self.timestamps_to_control_reg_changes = {}
        self.timestamps_to_control_group_events: dict[int, list[WaveformEvent]] = {}

        self.clock_name: str = None

    def enddefinitions(self, vcd, signals, cur_sig_vals):
        """
        Decide which signals we need to extract value change information from.
        signals and cur_sig_vals are unused variables; this function is inherited from VCDConverter.
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

        main_shortname = self.cell_metadata.main_shortname

        clock_filter = [n for n in names if n.endswith(f"{main_shortname}.clk")]
        if len(clock_filter) > 1:
            raise ProfilerException(f"Found multiple clocks: {clock_filter} Exiting...")
        elif len(clock_filter) == 0:
            raise ProfilerException("Can't find the clock? Exiting...")
        self.clock_name = clock_filter[0]
        # Depending on the simulator + OS, we may get different prefixes before the name
        # of the main component.
        self.signal_prefix = self.clock_name.split(f".{main_shortname}")[0]
        signal_id_dict[vcd.references_to_ids[self.clock_name]] = [self.clock_name]

        # replace the old key (cell_suffix) with the fully qualified cell name
        self.cell_metadata.add_signal_prefix(self.signal_prefix)
        # update fsms, par done registers, par groups with fully qualified names
        self.control_metadata.add_signal_prefix(self.signal_prefix)

        # get go and done for cells (the signals are exactly {cell}.go and {cell}.done)
        for cell in self.cell_metadata.cells:
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
            for fsm in self.control_metadata.fsms:
                if name.startswith(f"{fsm}.out["):
                    signal_id_dict[sid].append(name)
                if name.startswith(f"{fsm}.write_en") or name.startswith(f"{fsm}.in"):
                    tdcc_signal_id_to_names[sid].append(name)
            for par_done_reg in self.control_metadata.par_done_regs:
                if (
                    name.startswith(f"{par_done_reg}.in")
                    or name == f"{par_done_reg}.write_en"
                ):
                    tdcc_signal_id_to_names[sid].append(name)
            for ctrl_group_name in self.control_metadata.ctrl_groups:
                if name == f"{ctrl_group_name}_go_out":
                    control_signal_id_to_names[sid].append(name)

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
        vcd and cur_sig_vals are unused variables; this function is inherited from VCDConverter.
        """
        int_value = int(value, 2)
        if identifier_code in self.signal_id_to_names:
            signal_names = self.signal_id_to_names[identifier_code]

            for signal_name in signal_names:
                if (
                    signal_name == self.clock_name and int_value == 0
                ):  # ignore falling clock edges
                    continue
                event: WaveformEvent = WaveformEvent(signal_name, int_value)
                if time not in self.timestamps_to_events:
                    self.timestamps_to_events[time] = [event]
                else:
                    self.timestamps_to_events[time].append(event)
        if identifier_code in self.control_signal_id_to_names:
            signal_names = self.control_signal_id_to_names[identifier_code]
            for signal_name in signal_names:
                clean_signal_name = remove_size_from_name(signal_name).split("_go_out")[
                    0
                ]
                event = WaveformEvent(clean_signal_name, int_value)
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

    def postprocess(
        self,
        shared_cells_map: dict[str, dict[str, str]],
        control_metadata: ControlMetadata | None = None,
        utilization: Utilization | None = None,
    ):
        """
        Postprocess data mapping timestamps to events (signal changes)
        We have to postprocess instead of processing signals in a stream because
        signal changes that happen at the same time as a clock tick might be recorded
        *before* or *after* the clock change on the VCD file (hence why we can't process
        everything within a stream if we wanted to be precise)
        """

        # FIXME: This method would greatly benefit from some refactoring where we simply write separate logic for each probe type.

        clock_cycles = -1  # will be 0 on the 0th cycle
        started = False
        cell_active: set[str] = set()
        group_active: set[str] = set()
        structural_enable_active: set[str] = set()
        cell_enable_active = set()
        primitive_enable = set()

        # The events are "partial" because we don't know yet what the tid and pid would be.
        # (Will be filled in during create_timelines(); specifically in port_fsm_events())
        fsm_current = {fsm: 0 for fsm in self.control_metadata.fsms}  # fsm --> value

        probe_labels_to_sets = {
            "group_probe_out": group_active,
            "se_probe_out": structural_enable_active,
            "cell_probe_out": cell_enable_active,
            "primitive_probe_out": primitive_enable,
        }
        main_done = False  # Prevent creating a trace entry for the cycle where main.done is set high.
        for ts in self.timestamps_to_events:
            events = self.timestamps_to_events[ts]
            # NOTE: events is a list, so the `in` check is a linear scan. Might be worth creating a dictionary to manage events instead?
            started = (
                started
                or WaveformEvent(f"{self.cell_metadata.main_component}.go", 1) in events
            )
            if not started:  # only start counting when main component is on.
                continue
            # checking whether the timestamp has a rising edge
            if WaveformEvent(self.clock_name, 1) in events:
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
                if (
                    event.signal.endswith(".go") and event.value == 1
                ):  # cells have .go and .done
                    cell = event.signal.split(".go")[0]
                    cell_active.add(cell)
                    self.tracedata.cell_start_invoke(cell)
                if event.signal.endswith(".done") and event.value == 1:
                    cell = event.signal.split(".done")[0]
                    if (
                        cell == self.cell_metadata.main_component
                    ):  # if main is done, we shouldn't compute a "trace" for this cycle. set flag to True.
                        main_done = True
                    cell_active.remove(cell)
                # process fsms
                if ".out[" in event.signal:
                    fsm_name = event.signal.split(".out[")[0]
                    cell_name = ".".join(fsm_name.split(".")[:-1])
                    if fsm_current[fsm_name] != event.value:
                        # update value
                        fsm_current[fsm_name] = event.value
                # process all probes.
                for probe_label in probe_labels_to_sets:
                    cutoff = f"_{probe_label}"
                    if cutoff in event.signal:
                        # record cell name instead of component name.
                        split = event.signal.split(cutoff)[0].split(DELIMITER)[:-1]
                        cell_name = ".".join(
                            event.signal.split(cutoff)[0].split(".")[:-1]
                        )
                        split.append(cell_name)
                        probe_info = tuple(split)
                        if event.value == 1:
                            probe_labels_to_sets[probe_label].add(probe_info)
                        elif event.value == 0:
                            probe_labels_to_sets[probe_label].remove(probe_info)
            if not main_done:
                # accumulate cycles active for each cell that was active
                for cell in cell_active:
                    self.tracedata.register_cell_cycle(cell, clock_cycles)
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
                cycle_trace = (
                    create_cycle_trace(
                        self.cell_metadata,
                        info_this_cycle,
                        shared_cells_map,
                        True,
                    )
                    if utilization is None
                    else create_utilization_cycle_trace(
                        self.cell_metadata,
                        control_metadata,
                        info_this_cycle,
                        shared_cells_map,
                        True,
                        utilization,
                    )
                )  # True to track primitives
                self.tracedata.trace[clock_cycles] = cycle_trace
        self.clock_cycles = (
            clock_cycles  # last rising edge does not count as a full cycle (probably)
        )

    def postprocess_control(self):
        """
        Collects information on control register (fsm, pd) updates.
        Must run after self.postprocess() because this function relies on self.timestamps_to_clock_cycles
        (which gets filled in during self.postprocess()).
        """
        control_group_events: defaultdict[int, set[str]] = defaultdict(
            set
        )  # cycle count --> [control groups that are active that cycle]

        # FIXME: we might be able to get away with not computing this
        control_reg_per_cycle: dict[
            int, ControlRegUpdateType
        ] = {}  # clock cycle --> control_reg_update_type for leaf cell (longest cell name)

        # track when control groups were active
        control_group_start_cycles = {}
        for ts in self.timestamps_to_control_group_events:
            if ts in self.timestamps_to_clock_cycles:
                clock_cycle = self.timestamps_to_clock_cycles[ts]
                events = self.timestamps_to_control_group_events[ts]
                for event in events:
                    group_name = event.signal
                    if event.value == 1:  # control group started
                        control_group_start_cycles[group_name] = clock_cycle
                    elif event.value == 0:  # control group ended
                        active_range = range(
                            control_group_start_cycles[group_name], clock_cycle
                        )
                        del control_group_start_cycles[group_name]
                        self.tracedata.control_group_interval(group_name, active_range)
                        for i in active_range:
                            control_group_events[i].add(group_name)
        for k, v in control_group_start_cycles.items():
            end_cycle = len(self.tracedata.trace)
            for i in range(v, end_cycle):
                control_group_events[i].add(k)
        # track updates to control registers
        for ts in self.timestamps_to_control_reg_changes:
            if ts in self.timestamps_to_clock_cycles:
                clock_cycle = self.timestamps_to_clock_cycles[ts]
                events = self.timestamps_to_control_reg_changes[ts]
                cell_to_val_changes = {}
                # for each cell active in this clock cycle, what kinds of ctrl reg updates happened?
                # we will only store the ContrlRegUpdateType of the leaf cell (the cell on the top of the stack) since that's what is "currently active"
                # into control_reg_update_type
                # FIXME: is there a corner case I'm missing here?
                cell_to_change_type: dict[str, ControlRegUpdateType] = {}
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
                        reg_name in self.control_metadata.par_done_regs
                        and reg_new_value == 0
                    ):  # ignore when pd values turn 0 since they are only useful when they are high
                        upd = f"{write_en_split[-2]}:{reg_new_value}"
                        if cell_name in cell_to_val_changes:
                            cell_to_val_changes[cell_name] += f", {upd}"
                        else:
                            cell_to_val_changes[cell_name] = upd

                        cell_to_change_type[cell_name] = get_new_cell_to_change_type(
                            reg_name, cell_name, cell_to_change_type
                        )

                for cell in cell_to_val_changes:
                    self.tracedata.register_control_reg_update(
                        cell, clock_cycle, cell_to_val_changes[cell]
                    )
                if len(cell_to_change_type) > 0:
                    leaf_cell = sorted(
                        cell_to_change_type.keys(), key=(lambda k: k.count("."))
                    )[-1]
                    control_reg_per_cycle[clock_cycle] = cell_to_change_type[leaf_cell]
        return (control_group_events, control_reg_per_cycle)


def create_cycle_trace(
    cell_info: CellMetadata,
    info_this_cycle: dict[str, str | dict[str, str]],
    shared_cell_map: dict[str, dict[str, str]],
    include_primitives: bool,
):
    """
    Creates a CycleTrace object for stack elements in this cycle, computing the dependencies between them.
    """
    assert cell_info is not None
    stacks_this_cycle: list[list[StackElement]] = []
    parents: set[str] = (
        set()
    )  # keeping track of entities that are parents of other entities
    elem_name_to_stack: dict[
        str, list[StackElement]
    ] = {}  # each unique group inv mapping to its stack. the "group" should be the last item on each stack
    main_shortname = cell_info.main_shortname
    elem_name_to_stack[cell_info.main_component] = [
        StackElement(main_shortname, StackElementType.CELL, is_main=True)
    ]
    cell_worklist = [cell_info.main_component]  # worklist of cell names
    while cell_worklist:
        current_cell = cell_worklist.pop()
        # catch all active units that are groups in this component.
        active_groups: set[str] = (
            info_this_cycle["group-active"][current_cell]
            if current_cell in info_this_cycle["group-active"]
            else set()
        )
        structural_enables: set[str] = (
            info_this_cycle["structural-enable"][current_cell]
            if current_cell in info_this_cycle["structural-enable"]
            else set()
        )
        primitive_enables: set[str] = (
            info_this_cycle["primitive-enable"][current_cell]
            if current_cell in info_this_cycle["primitive-enable"]
            else set()
        )
        cell_invokes = (
            info_this_cycle["cell-invoke"][current_cell]
            if current_cell in info_this_cycle["cell-invoke"]
            else dict()
        )

        # obtain and process control enables
        add_control_enables(
            current_cell, active_groups, structural_enables, elem_name_to_stack, parents
        )

        # get all of the other active units
        add_structural_enables(
            current_cell, structural_enables, elem_name_to_stack, parents
        )

        # get primitives if requested.
        if include_primitives:
            add_primitives(
                current_cell,
                primitive_enables,
                elem_name_to_stack,
                parents,
                shared_cell_map,
                cell_info.get_component_of_cell(current_cell),
            )

        # by this point, we should have covered all groups in the same component...
        # now we need to construct stacks for any cells that are called from a group in the current component.
        invoked_cells: list[str] = add_invoked_cells(
            current_cell,
            cell_info,
            shared_cell_map,
            cell_invokes,
            info_this_cycle["cell-active"],
            elem_name_to_stack,
            parents,
        )
        cell_worklist += invoked_cells

    # Only retain stacks that lead to leaf nodes.
    for elem in elem_name_to_stack:
        if elem not in parents:
            stacks_this_cycle.append(elem_name_to_stack[elem])

    return CycleTrace(stacks_this_cycle)


def create_utilization_cycle_trace(
    cell_info: CellMetadata,
    control_metadata: ControlMetadata,
    info_this_cycle: dict[str, str | dict[str, str]],
    shared_cell_map: dict[str, dict[str, str]],
    include_primitives: bool,
    utilization: Utilization,
):
    """
    Creates a UtilizationCycleTrace object for stack elements in this cycle, computing the dependencies between them.
    """
    cycle_trace = create_cycle_trace(
        cell_info, info_this_cycle, shared_cell_map, include_primitives
    )
    return UtilizationCycleTrace(utilization, control_metadata, cycle_trace.stacks)


def add_control_enables(
    cell_name: str,
    active_groups: set[str],
    structural_enables: set[str],
    elem_name_to_stack: dict[str, list[StackElement]],
    parents: set[str],
):
    """
    Helper function for create_cycle_trace(). Processes groups enabled by control in `cell_name` in this cycle.

    Updates:
        - elem_name_to_stack: Adds a mapping from enabled groups' names to their stacks
        - parents: Registers `cell_name` as a parent
    """
    for active_unit in active_groups.difference(structural_enables):
        shortname = active_unit.split(".")[-1]
        elem_name_to_stack[active_unit] = elem_name_to_stack[cell_name] + [
            StackElement(shortname, StackElementType.GROUP)
        ]
        parents.add(cell_name)


def add_structural_enables(
    cell_name: str,
    structural_enables: set[str],
    elem_name_to_stack: dict[str, list[StackElement]],
    parents: set[str],
):
    """
    Helper function for create_cycle_trace(). Processes groups that are structurally enabled by other groups in `cell_names` in this cycle.

    Updates:
        - elem_name_to_stack: Adds a mapping from enabled groups' names to their stacks
        - parents: Registers any parent groups (groups that structurally enable other groups) as a parent
    """

    covered_se: set[str] = set()
    while len(covered_se) < len(structural_enables):
        # loop through all other elements to figure out parent child info (structural enables)
        for group in structural_enables:
            if group in elem_name_to_stack:
                # the group has been processed already
                continue
            shortname = group.split(".")[-1]
            for parent_group in structural_enables[group]:
                parent = f"{cell_name}.{parent_group}"
                # if parent is not present, it means that the parent is also structurally enabled, and hasn't been processed yet
                if parent in elem_name_to_stack:
                    elem_name_to_stack[group] = elem_name_to_stack[parent] + [
                        StackElement(shortname, StackElementType.GROUP)
                    ]
                    covered_se.add(group)
                    parents.add(parent)


def add_primitives(
    current_cell: str,
    primitive_enables: set[str],
    elem_name_to_stack: dict[str, list[StackElement]],
    parents: set[str],
    shared_cell_map: dict[str, dict[str, str]],
    component: str,
):
    """
    Helper function called by create_cycle_trace(). Processes primitives active this cycle in `current_cell` to the stack.

    Updates:
        - elem_name_to_stack: Adds a mapping from the primitive's name to the primitive's stack
        - parents: Registers the primitive's caller group as a parent
    """
    for primitive_parent_group in primitive_enables:
        for primitive_name in primitive_enables[primitive_parent_group]:
            primitive_parent = f"{current_cell}.{primitive_parent_group}"
            primitive_shortname = primitive_name.split(".")[-1]
            elem_name_to_stack[primitive_name] = elem_name_to_stack[
                primitive_parent
            ] + [
                StackElement(
                    primitive_shortname,
                    StackElementType.PRIMITIVE,
                    replacement_cell_name=shared_cell_map[component][
                        primitive_shortname
                    ]
                    if component in shared_cell_map
                    and primitive_shortname in shared_cell_map[component]
                    else None,
                )
            ]
            parents.add(primitive_parent)


def add_invoked_cells(
    cell_name: str,
    cell_info: CellMetadata,
    shared_cell_map: dict[str, dict[str, str]],
    cell_invokes: dict[str, dict[str, str]],
    active_cells: set[str],
    elem_name_to_stack: dict[str, list[StackElement]],
    parents: set[str],
):
    """
    Helper function called by create_cycle_trace(). Processes cells invoked from cell `cell_name` in this cycle to the stack.

    Updates:
        - elem_name_to_stack: Adds a mapping from the cell's name to the cell's stack
        - parents: Registers the invoker group as a parent
    """
    invoked_cells = []
    for cell_invoker_group in cell_invokes:
        for invoked_cell in cell_invokes[cell_invoker_group]:
            # TODO: if rewritten... then look for the rewritten cell from cell-active
            # probably worth putting some info in the flame graph that the cell is rewritten from the originally coded one?
            current_component = (
                cell_info.get_component_of_cell(cell_name)
                if cell_name != cell_info.main_component
                else cell_info.main_shortname
            )
            cell_split = invoked_cell.split(".")
            cell_shortname = cell_split[-1]
            cell_prefix = ".".join(cell_split[:-1])
            if (
                current_component in shared_cell_map
                and cell_shortname in shared_cell_map[current_component]
            ):
                replacement_cell_shortname = shared_cell_map[current_component][
                    cell_shortname
                ]
                replacement_cell = f"{cell_prefix}.{replacement_cell_shortname}"
                if replacement_cell not in active_cells:
                    raise ProfilerException(
                        f"Replacement cell {replacement_cell_shortname} for cell {invoked_cell} not found in active cells list!\n{active_cells}"
                    )
                invoked_cells.append(replacement_cell)
                cell_component = cell_info.get_component_of_cell(replacement_cell)
                parent = f"{cell_name}.{cell_invoker_group}"
                elem_name_to_stack[replacement_cell] = elem_name_to_stack[parent] + [
                    StackElement(
                        cell_shortname,
                        StackElementType.CELL,
                        component_name=cell_component,
                        replacement_cell_name=replacement_cell_shortname,
                    )
                ]
                parents.add(parent)
            elif invoked_cell in active_cells:
                invoked_cells.append(invoked_cell)
                cell_component = cell_info.get_component_of_cell(invoked_cell)
                parent = f"{cell_name}.{cell_invoker_group}"
                elem_name_to_stack[invoked_cell] = elem_name_to_stack[parent] + [
                    StackElement(
                        cell_shortname,
                        StackElementType.CELL,
                        component_name=cell_component,
                    )
                ]
                parents.add(parent)

    return invoked_cells


def get_new_cell_to_change_type(
    reg_name: str, cell_name: str, cell_to_change_type: dict[str, ControlRegUpdateType]
):
    """
    Returns a new value of `cell_name`'s ControlRegUpdateType, based on the updated control register and the current ControlRegUpdateType.

    reg_name: the control register that just got updated
    cell_name: the cell from which the control register comes from
    cell_to_change_type: dict containing current values of ControlRegUpdateTypes.
    """
    par_done_indicator = ".pd"
    fsm_indicator = ".fsm"
    if cell_name not in cell_to_change_type:
        if par_done_indicator in reg_name:
            return ControlRegUpdateType.PAR_DONE
        elif fsm_indicator in reg_name:
            return ControlRegUpdateType.FSM
    elif (
        par_done_indicator in reg_name
        and cell_to_change_type[cell_name] == ControlRegUpdateType.FSM
    ):
        return ControlRegUpdateType.BOTH
    elif (
        fsm_indicator in reg_name
        and cell_to_change_type[cell_name] == ControlRegUpdateType.PAR_DONE
    ):
        return ControlRegUpdateType.BOTH


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
