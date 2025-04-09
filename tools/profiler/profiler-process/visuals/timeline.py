import json
import os

ts_multiplier = 1  # [timeline view] ms on perfetto UI that resembles a single cycle
JSON_INDENT = "    "  # [timeline view] indentation for generating JSON on the fly
num_timeline_events = 0  # [timeline view] recording how many events have happened


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

    def get_metatrack_pid_tid(self, fsm_name):
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
        #     (fsm_pid, fsm_tid) = cell_to_info[cell_name].get_metatrack_pid_tid(fsm_name)
        #     for entry in partial_fsm_events[fsm_name]:
        #         entry["pid"] = fsm_pid
        #         entry["tid"] = fsm_tid
        #         write_timeline_event(entry, out_file)
        del partial_fsm_events[fsm_name]
    for cycle, update in control_updates[cell_name]:
        (control_pid, control_tid) = cell_to_info[cell_name].get_metatrack_pid_tid(
            "CTRL"
        )
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
