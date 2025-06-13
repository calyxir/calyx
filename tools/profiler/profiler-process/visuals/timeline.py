import json
import os

from classes import (
    TraceData,
    ControlRegUpdates,
    StackElementType,
    CellMetadata,
    PathMetadata
)

ts_multiplier = 1  # [timeline view] ms on perfetto UI that resembles a single cycle
JSON_INDENT = "    "  # [timeline view] indentation for generating JSON on the fly
num_timeline_events = 0  # [timeline view] recording how many events have happened


def setup_enable_to_tid(path_metadata: dict[str, int] | None , starter_idx):
    return {enable: path_metadata[enable] + starter_idx for enable in path_metadata} if path_metadata else {}

class TimelineCell:
    """
    Bookkeeping for forming cells and their groups

    Current system:
    FIXME: we are assuming that there are no nested pars.
    tid 1 is reserved for the cell itself
    tid 2 is reserved for control register updates
    tid 3+ will be computed using the path descriptor
    """
    def __init__(self, name: str, pid: int, path_metadata: dict[str, int] | None =None):
        self.name: str = name
        self.pid: int = pid
        self.tid: int = 1
        self.control_tid: int = 2
        # basically path_metadata info but all ids are bumped by 3 (since path identifiers start from 0)
        self.enable_to_tid = setup_enable_to_tid(path_metadata, 3)
        self.currently_active_group_to_tid = {}
        self.queued_tids = []

    @property
    def control_pid_tid(self):
        # metatrack is the second tid, containing information about control register updates
        return (self.pid, self.control_tid)

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
    """
    Output a event to the timeline JSON.
    """
    global num_timeline_events
    if num_timeline_events == 0:  # shouldn't prepend a comma on the first entry
        out_file.write(f"\n{JSON_INDENT}{json.dumps(event)}")
    else:
        out_file.write(f",\n{JSON_INDENT}{json.dumps(event)}")
    num_timeline_events += 1


def port_control_events(
    control_updates: dict[str, list[ControlRegUpdates]],
    cell_to_info: dict[str, TimelineCell],
    cell_name: str,
    out_file,
):
    """
    Add control events to the timeline (values are already determined, this
    function just sets the pid and tid, and writes to file).
    """
    if cell_name not in control_updates:
        # cells that are not in control_updates do not have any control register updates
        # they are probably single-group components.
        return
    for update_info in control_updates[cell_name]:
        (control_pid, control_tid) = cell_to_info[cell_name].get_metatrack_pid_tid(
            "CTRL"
        )
        begin_event = {
            "name": update_info.updates,
            "cat": "CTRL",
            "ph": "B",
            "ts": update_info.clock_cycle * ts_multiplier,
            "pid": control_pid,
            "tid": control_tid,
        }
        end_event = {
            "name": update_info.updates,
            "cat": "CTRL",
            "ph": "E",
            "ts": (update_info.clock_cycle + 1) * ts_multiplier,
            "pid": control_pid,
            "tid": control_tid,
        }
        write_timeline_event(begin_event, out_file)
        write_timeline_event(end_event, out_file)
    del control_updates[cell_name]


def compute_timeline(tracedata: TraceData, cell_metadata: CellMetadata, path_metadata: PathMetadata, out_dir):
    """
    Compute and output a JSON that conforms to the Google Trace File format.
    Each cell gets its own process id, where tid 1 is the duration of the cell itself,
    tid 2 contains control register updates, and tid 3+ contains durations of groups.
    """
    # generate the JSON on the fly instead of storing everything in a list to save memory
    out_path = os.path.join(out_dir, "timeline-dump.json")
    out_file = open(out_path, "w", encoding="utf-8")
    # start the JSON file
    out_file.write(f'{{\n{JSON_INDENT}"traceEvents": [')
    # each cell gets its own pid. The cell's lifetime is tid 1, followed by the FSM(s), then groups
    # main component gets pid 1
    main_path_metadata: dict[str, int] = path_metadata.component_to_paths[cell_metadata.main_shortname]
    cell_to_info: dict[str, TimelineCell] = {
        cell_metadata.main_component: TimelineCell(cell_metadata.main_component, 1, path_metadata=main_path_metadata)
    }
    # generate JSON for all FSM events in main
    port_control_events(
        tracedata.control_reg_updates,
        cell_to_info,
        cell_metadata.main_component,
        out_file,
    )
    group_to_parent_cell = {}
    pid_acc = 2
    currently_active = set()
    for i in tracedata.trace:
        active_this_cycle: set[tuple[str, str]] = set()
        for stack in tracedata.trace[i].stacks:
            stack_acc = cell_metadata.main_component
            current_cell = (
                cell_metadata.main_component
            )  # need to keep track of cells in case we have a structural group enable.
            display_name = None
            for stack_elem in stack:
                match stack_elem.element_type:
                    case StackElementType.CELL:
                        if stack_elem.is_main:
                            # don't accumulate to the stack if your name is main.
                            name = cell_metadata.main_component
                        else:
                            display_name = f"{stack_acc}.{stack_elem.internal_name}"
                            if stack_elem.replacement_cell_name is not None:
                                # shared cell. use the info of the replacement cell
                                display_name += f" ({stack_elem.replacement_cell_name})"
                                stack_acc += "." + stack_elem.replacement_cell_name
                            else:
                                stack_acc += "." + stack_elem.internal_name
                            name = stack_acc
                            current_cell = name
                            if name not in cell_to_info:  # cell is not registered yet
                                cell_component = cell_metadata.get_component_of_cell(name)
                                if cell_component in path_metadata.component_to_paths:
                                    component_pathdata = path_metadata.component_to_paths[cell_component]
                                    cell_to_info[name] = TimelineCell(name, pid_acc, component_pathdata=component_pathdata)
                                else:
                                    cell_to_info[name] = TimelineCell(name, pid_acc)
                                # generate JSON for all FSM events in this cell
                                port_control_events(
                                    tracedata.control_reg_updates,
                                    cell_to_info,
                                    name,
                                    out_file,
                                )
                                pid_acc += 1
                    case StackElementType.PRIMITIVE:
                        # ignore primitives for now
                        continue
                    case StackElementType.GROUP:
                        name = stack_acc + "." + stack_elem.internal_name
                        group_to_parent_cell[name] = current_cell
                active_this_cycle.add((name, display_name))
        for nonactive_element, nonactive_element_display in currently_active.difference(
            active_this_cycle
        ):  # element that was previously active but no longer is.
            # make end event
            end_event = create_timeline_event(
                nonactive_element,
                i,
                "E",
                cell_to_info,
                group_to_parent_cell,
                display_name=nonactive_element_display,
            )
            write_timeline_event(end_event, out_file)
        for newly_active_element, newly_active_display in active_this_cycle.difference(
            currently_active
        ):  # element that started to be active this cycle.
            begin_event = create_timeline_event(
                newly_active_element,
                i,
                "B",
                cell_to_info,
                group_to_parent_cell,
                display_name=newly_active_display,
            )
            write_timeline_event(begin_event, out_file)
        currently_active = active_this_cycle

    # Read through all cycles; postprocessing
    for (
        still_active_element,
        still_active_display,
    ) in (
        currently_active
    ):  # need to close any elements that are still active at the end of the simulation
        end_event = create_timeline_event(
            still_active_element,
            len(tracedata.trace),
            "E",
            cell_to_info,
            group_to_parent_cell,
            display_name=still_active_display,
        )
        write_timeline_event(end_event, out_file)

    # close off the json
    out_file.write("\t\t]\n}")
    out_file.close()


def create_timeline_event(
    element_name,
    cycle,
    event_type,
    cell_to_info,
    group_to_parent_cell,
    display_name=None,
):
    """
    Creates a JSON entry for traceEvents.
    element_name: fully qualified name of cell/group
    cycle: timestamp of the event, in cycles
    event_type: "B" for begin event, "E" for end event
    display_name: Optional arg for when we want the name of a cell entry to be something else (ex. shared cells). Ignored for groups
    """
    if element_name in cell_to_info:  # cell
        event = {
            "name": element_name if display_name is None else display_name,
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
