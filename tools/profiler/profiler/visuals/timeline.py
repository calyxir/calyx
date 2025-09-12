from dataclasses import dataclass
import json
import os

import uuid

from perfetto.trace_builder.proto_builder import TraceProtoBuilder
from perfetto.protos.perfetto.trace.perfetto_trace_pb2 import (
    TrackEvent,
)

from enum import Enum


from profiler.classes.tracedata import (
    TraceData,
    ControlRegUpdates,
    StackElementType,
)
from profiler.classes.cell_metadata import CellMetadata

ts_multiplier = 1  # [timeline view] ms on perfetto UI that resembles a single cycle
JSON_INDENT = "    "  # [timeline view] indentation for generating JSON on the fly
num_timeline_events = 0  # [timeline view] recording how many events have happened


def setup_enable_to_tid(
    enable_to_threadid: dict[str, int] | None, starter_idx
) -> dict[str, int]:
    return (
        {
            enable: enable_to_threadid[enable] + starter_idx
            for enable in enable_to_threadid
        }
        if enable_to_threadid
        else {}
    )


class EventType(Enum):
    START = 1
    END = 2


@dataclass
class ProtoTimelineCell:
    builder: TraceProtoBuilder
    fully_qualified_cell_name: str
    sid_name: str
    sid_val: int
    enable_to_thread: dict[str, int]
    enable_id_to_uuid: dict[str, int]
    control_group_id_to_uuid: dict[str, int]

    def __init__(
        self,
        builder: TraceProtoBuilder,
        fully_qualified_cell_name: str,
        sid: int,
        enable_to_thread: dict[str, int],
    ):
        self.builder = builder
        self.fully_qualified_cell_name = fully_qualified_cell_name
        self.sid_name = self.fully_qualified_cell_name.replace(".", "_").upper()
        self.sid_val = sid
        self.events = list()
        self.enable_to_thread = enable_to_thread
        self.enable_id_to_uuid = {}
        self.control_group_id_to_uuid = {}
        self.cell_uuid = self._define_track(fully_qualified_cell_name)
        self.control_uuid = self._define_track(
            "Control Groups", parent_track_uuid=self.cell_uuid
        )
        self.control_reg_uuid = self._define_track(
            "Control Register Updates", parent_track_uuid=self.cell_uuid
        )
        # groups that did not get an assigned threadid
        self.misc_group_uuid = self._define_track(
            "Non-id-ed groups", parent_track_uuid=self.cell_uuid
        )
        # # convert threadids to uuids
        # threadid_to_uuid: dict[int, int] = {}
        # for group, threadid in enable_to_thread.items():
        #     if threadid not in threadid_to_uuid:
        #         threadid_to_uuid[threadid] = self._define_track(
        #             f"Thread {threadid:03}", parent_track_uuid=self.cell_uuid
        #         )
        #     self.enable_id_to_uuid[group] = threadid_to_uuid[threadid]

        # self.group_uuid = self._define_track("Groups", self.cell_uuid)

    # Helper to define a new track with a unique UUID
    def _define_track(self, track_name, parent_track_uuid=None):
        track_uuid = uuid.uuid4().int & ((1 << 63) - 1)
        packet = self.builder.add_packet()
        packet.track_descriptor.uuid = track_uuid
        packet.track_descriptor.name = track_name  # self.fully_qualified_cell_name
        if parent_track_uuid:
            packet.track_descriptor.parent_uuid = parent_track_uuid
        self.enable_id_to_uuid[track_name] = track_uuid

        return track_uuid

    # Helper to add a begin or end slice event to a specific track
    def _add_slice_event(self, ts, event_type, event_track_uuid, name):
        packet = self.builder.add_packet()
        packet.timestamp = ts
        packet.track_event.type = event_type
        packet.track_event.track_uuid = event_track_uuid
        if name:
            packet.track_event.name = name
        packet.trusted_packet_sequence_id = self.sid_val

    def register_cell_event(self, timestamp: int, event_type: TrackEvent.Type):
        self._add_slice_event(
            timestamp, event_type, self.cell_uuid, self.fully_qualified_cell_name
        )

    def register_group_event(
        self, enable_id: str, timestamp: int, event_type: TrackEvent.Type
    ):
        # NOTE: enable_id is not fully qualified.
        group_name = enable_id.split("UG")[0]
        #     if threadid not in threadid_to_uuid:
        #         threadid_to_uuid[threadid] = self._define_track(
        #             f"Thread {threadid:03}", parent_track_uuid=self.cell_uuid
        #         )

        if enable_id in self.enable_to_thread:
            if enable_id in self.enable_id_to_uuid:
                uuid = self.enable_id_to_uuid[enable_id]
                self._add_slice_event(timestamp, event_type, uuid, group_name)
            else:
                thread_id = self.enable_to_thread[enable_id]
                uuid = self._define_track(
                    f"Thread {thread_id:03}", parent_track_uuid=self.cell_uuid
                )
                self._add_slice_event(timestamp, event_type, uuid, group_name)
                self.enable_id_to_uuid[enable_id] = uuid
        else:
            self._add_slice_event(
                timestamp, event_type, self.misc_group_uuid, group_name
            )

    def register_control_register_event(
        self, updates: str, timestamp: int, event_type: TrackEvent.Type
    ):
        self._add_slice_event(timestamp, event_type, self.control_reg_uuid, updates)

    def register_control_event(
        self,
        ctrl_group: str,
        timestamp: int,
        event_type: TrackEvent.Type,
    ):
        # use source locations when available. FIXME: make this less hacky
        if "~ " in ctrl_group:
            name = ctrl_group.split("~ ")[1].split("(")[0]
        else:
            name = ctrl_group.split("(")[0]
        if ctrl_group not in self.control_group_id_to_uuid:
            uuid = self._define_track(f"Control Group: {name}", self.control_uuid)
            self.control_group_id_to_uuid[ctrl_group] = uuid
        else:
            uuid = self.control_group_id_to_uuid[ctrl_group]
        # NOTE: ctrl_group is not fully qualified.

        self._add_slice_event(timestamp, event_type, uuid, name)


@dataclass
class ProtoTimeline:
    builder: TraceProtoBuilder
    tracedata: TraceData
    enable_thread_data: dict[str, dict[str, int]]
    cell_infos: dict[str, ProtoTimelineCell]
    sid_acc: int

    def __init__(
        self,
        tracedata: TraceData,
        cell_metadata: CellMetadata,
        enable_thread_data: dict[str, dict[str, int]],
    ):
        self.builder = TraceProtoBuilder()
        self.tracedata = tracedata
        self.cell_metadata = cell_metadata
        self.enable_thread_data = enable_thread_data
        self.cell_infos = {}
        self.sid_acc = 300  # some arbitrary number

    def add_cell_if_not_present(self, cell):
        if cell not in self.cell_infos:
            cell_component = self.cell_metadata.get_component_of_cell(cell)
            self.cell_infos[cell] = ProtoTimelineCell(
                self.builder,
                cell,
                self.sid_acc,
                self.enable_thread_data[cell_component],
            )
            self.sid_acc += 1
            self.port_register_updates(cell)

    def port_register_updates(self, cell_name: str):
        if cell_name not in self.tracedata.control_reg_updates:
            # cells that are not in control_updates do not have any control register updates
            # they are probably single-group components.
            return
        for update_info in self.tracedata.control_reg_updates[cell_name]:
            self.cell_infos[cell_name].register_control_register_event(
                update_info.updates,
                update_info.clock_cycle * ts_multiplier,
                TrackEvent.TYPE_SLICE_BEGIN,
            )
            self.cell_infos[cell_name].register_control_register_event(
                update_info.updates,
                (update_info.clock_cycle + 1) * ts_multiplier,
                TrackEvent.TYPE_SLICE_END,
            )

        # uncomment only if/when we remove the JSON-based timeline.
        # del control_updates[cell_name]

    def register_cell_event(self, cell, timestamp, event_type):
        self.add_cell_if_not_present(cell)
        self.cell_infos[cell].register_cell_event(timestamp, event_type)

    def register_control_event(
        self,
        fully_qualified_ctrl_group: str,
        timestamp: int,
        event_type: TrackEvent.Type,
    ):
        name_split = fully_qualified_ctrl_group.split(".")
        cell = ".".join(name_split[:-1])
        name = name_split[-1]
        self.add_cell_if_not_present(cell)
        self.cell_infos[cell].register_control_event(name, timestamp, event_type)

    def register_event(
        self, fully_qualified_group: str, timestamp: int, event_type: TrackEvent.Type
    ):
        name_split = fully_qualified_group.split(".")
        cell = ".".join(name_split[:-1])
        name = name_split[-1]
        self.add_cell_if_not_present(cell)
        self.cell_infos[cell].register_group_event(name, timestamp, event_type)

    def emit(self, output_filename):
        with open(output_filename, "wb") as f:
            f.write(self.builder.serialize())


def compute_protobuf_timeline(
    tracedata: TraceData,
    cell_metadata: CellMetadata,
    enable_thread_data: dict[str, dict[str, int]],
    out_dir: str,
):
    proto: ProtoTimeline = ProtoTimeline(tracedata, cell_metadata, enable_thread_data)

    currently_active_ctrl_groups: set[str] = set()
    currently_active_cells: set[str] = set()
    currently_active_groups: set[str] = set()

    for i in tracedata.trace_with_control_groups:
        this_cycle_active_ctrl_groups: set[str] = set()
        this_cycle_active_cells: set[str] = set()
        this_cycle_active_groups: set[str] = set()
        for stack in tracedata.trace_with_control_groups[i].stacks:
            stack_acc = cell_metadata.main_component
            for stack_elem in stack:
                match stack_elem.element_type:
                    case StackElementType.CELL:
                        if not stack_elem.is_main:
                            stack_acc = f"{stack_acc}.{stack_elem.name}"
                        this_cycle_active_cells.add(stack_acc)
                    case StackElementType.CONTROL_GROUP:
                        this_cycle_active_ctrl_groups.add(f"{stack_acc}.{stack_elem}")
                    case StackElementType.GROUP:
                        this_cycle_active_groups.add(
                            f"{stack_acc}.{stack_elem.internal_name}"
                        )

        # cells

        for done_cell in currently_active_cells.difference(this_cycle_active_cells):
            proto.register_cell_event(done_cell, i, TrackEvent.TYPE_SLICE_END)

        for new_cell in this_cycle_active_cells.difference(currently_active_cells):
            proto.register_cell_event(new_cell, i, TrackEvent.TYPE_SLICE_BEGIN)

        # control groups

        for gone_ctrl_group in sorted(
            currently_active_ctrl_groups.difference(this_cycle_active_ctrl_groups)
        ):
            proto.register_control_event(gone_ctrl_group, i, TrackEvent.TYPE_SLICE_END)

        for new_ctrl_group in sorted(
            this_cycle_active_ctrl_groups.difference(currently_active_ctrl_groups)
        ):
            proto.register_control_event(new_ctrl_group, i, TrackEvent.TYPE_SLICE_BEGIN)

        # normal groups

        for done_group in currently_active_groups.difference(this_cycle_active_groups):
            proto.register_event(done_group, i, TrackEvent.TYPE_SLICE_END)

        for new_group in this_cycle_active_groups.difference(currently_active_groups):
            proto.register_event(new_group, i, TrackEvent.TYPE_SLICE_BEGIN)

        # update
        currently_active_cells = this_cycle_active_cells
        currently_active_ctrl_groups = this_cycle_active_ctrl_groups
        currently_active_groups = this_cycle_active_groups

    # elements that are active until the very end

    for active_at_end_cell in currently_active_cells:
        proto.register_cell_event(active_at_end_cell, i + 1, TrackEvent.TYPE_SLICE_END)

    for active_at_end_ctrl_group in currently_active_ctrl_groups:
        proto.register_control_event(
            active_at_end_ctrl_group, i + 1, TrackEvent.TYPE_SLICE_END
        )

    for active_at_end_group in currently_active_groups:
        proto.register_event(active_at_end_group, i + 1, TrackEvent.TYPE_SLICE_END)

    out_path = os.path.join(out_dir, "timeline_trace.pftrace")
    proto.emit(out_path)


class TimelineCell:
    """
    Bookkeeping for forming cells and their groups

    Current system:
    FIXME: we are assuming that there are no nested pars.
    tid 1 is reserved for the cell itself
    tid 2 is reserved for control register updates
    tid 3+ will be computed using the path descriptor
    """

    def __init__(
        self, name: str, pid: int, enable_to_threadid: dict[str, int] | None = None
    ):
        self.name: str = name
        self.pid: int = pid
        self.tid: int = 1
        self.control_tid: int = 2
        # basically path_metadata info but all ids are bumped by 3 (since path identifiers start from 0)
        self.enable_to_tid: dict[str, int] = setup_enable_to_tid(enable_to_threadid, 3)
        self.misc_enable_acc = (
            1000  # compiler-generated groups that weren't recorded in enable_to_tid
        )
        # FIXME: this value ought to be accessed through a variable and really not as a hardcoded value. but probably ok for a first pass
        self.unique_group_str = "UG"

    @property
    def control_pid_tid(self):
        # metatrack is the second tid, containing information about control register updates
        return (self.pid, self.control_tid)

    def add_group(self, enable_name: str):
        group_name = enable_name.split(self.unique_group_str)[0]
        if enable_name in self.enable_to_tid:
            group_tid = self.enable_to_tid[enable_name]
        else:
            # this has to be a structural enable. not sure what the best behavior here is
            group_tid = self.misc_enable_acc
            self.enable_to_tid[group_name] = group_tid
            self.misc_enable_acc += 1
        return (self.pid, group_tid, group_name)

    def remove_group(self, enable_name):
        group_name = enable_name.split(self.unique_group_str)[0]
        group_tid = self.enable_to_tid[enable_name]
        # del self.currently_active_group_to_tid[group_name]
        return (self.pid, group_tid, group_name)


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
        (control_pid, control_tid) = cell_to_info[cell_name].control_pid_tid
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


@dataclass(frozen=True)
class ActiveCell:
    cell_name: str
    display_name: str | None

    @property
    def name(self) -> str:
        return self.cell_name if self.display_name is None else self.display_name


@dataclass(frozen=True)
class ActiveEnable:
    enable_name: str
    cell_name: str  # cell from which enable is active from


def compute_timeline(
    tracedata: TraceData,
    cell_metadata: CellMetadata,
    enable_thread_data: dict[str, dict[str, int]],
    out_dir,
):
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
    cell_to_info: dict[str, TimelineCell] = {
        cell_metadata.main_component: TimelineCell(
            cell_metadata.main_component,
            1,
            enable_to_threadid=enable_thread_data[cell_metadata.main_shortname],
        )
    }
    # generate JSON for all FSM events in main
    port_control_events(
        tracedata.control_reg_updates,
        cell_to_info,
        cell_metadata.main_component,
        out_file,
    )
    pid_acc = 2
    currently_active_cells: set[ActiveCell] = set()
    currently_active_groups: set[ActiveEnable] = set()
    for i in tracedata.trace:
        cells_active_this_cycle: set[ActiveCell] = set()
        groups_active_this_cycle: set[ActiveEnable] = set()
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
                                cell_component = cell_metadata.get_component_of_cell(
                                    name
                                )
                                if cell_component in enable_thread_data:
                                    cell_to_info[name] = TimelineCell(
                                        name,
                                        pid_acc,
                                        enable_to_threadid=enable_thread_data[
                                            cell_component
                                        ],
                                    )
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
                        cells_active_this_cycle.add(ActiveCell(name, display_name))
                    case StackElementType.PRIMITIVE:
                        # ignore primitives for now
                        continue
                    case StackElementType.GROUP:
                        # TODO: maybe we need to retain stack names? Reevaluate this commenting out
                        # name = stack_acc + "." + stack_elem.internal_name
                        groups_active_this_cycle.add(
                            ActiveEnable(stack_elem.internal_name, current_cell)
                        )

        register_done_elements_for_cycle(
            out_file,
            cell_to_info,
            currently_active_cells,
            currently_active_groups,
            i,
            cells_active_this_cycle,
            groups_active_this_cycle,
        )

        register_new_elements(
            out_file,
            cell_to_info,
            currently_active_cells,
            currently_active_groups,
            i,
            cells_active_this_cycle,
            groups_active_this_cycle,
        )

        currently_active_cells = cells_active_this_cycle
        currently_active_groups = groups_active_this_cycle

    # Gotten through all cycles; postprocessing any cells and groups that were active until the very end
    # need to close any elements that are still active at the end of the simulation
    for still_active_cell in currently_active_cells:
        cell_end_event = create_cell_timeline_event(
            still_active_cell, len(tracedata.trace), "E", cell_to_info
        )
        write_timeline_event(cell_end_event, out_file)
    for still_active_group in currently_active_groups:
        group_end_event = create_group_timeline_event(
            still_active_group, len(tracedata.trace), "E", cell_to_info
        )
        write_timeline_event(group_end_event, out_file)

    # close off the json
    out_file.write("\t\t]\n}")
    out_file.close()


def register_new_elements(
    out_file,
    cell_to_info,
    currently_active_cells,
    currently_active_groups,
    i,
    cells_active_this_cycle,
    groups_active_this_cycle,
):
    """
    Identifies and creates events for cells/group enables that started execution this cycle.
    """
    for newly_active_cell in cells_active_this_cycle.difference(currently_active_cells):
        # cell that started to be active this cycle
        cell_begin_event = create_cell_timeline_event(
            newly_active_cell, i, "B", cell_to_info
        )
        write_timeline_event(cell_begin_event, out_file)
    for newly_active_group in groups_active_this_cycle.difference(
        currently_active_groups
    ):
        # group that started to be active this cycle
        group_start_event = create_group_timeline_event(
            newly_active_group, i, "B", cell_to_info
        )
        write_timeline_event(group_start_event, out_file)


def register_done_elements_for_cycle(
    out_file,
    cell_to_info,
    currently_active_cells,
    currently_active_groups,
    i,
    cells_active_this_cycle,
    groups_active_this_cycle,
):
    """
    Identifies and creates events for cells/group enables that finished execution this cycle.
    """
    for nonactive_cell in currently_active_cells.difference(cells_active_this_cycle):
        # cell that was previously active but no longer is
        # make end event
        cell_end_event = create_cell_timeline_event(
            nonactive_cell, i, "E", cell_to_info
        )
        write_timeline_event(cell_end_event, out_file)
    for nonactive_group in currently_active_groups.difference(groups_active_this_cycle):
        # group/enable that was previously active but no longer is
        # make end event
        group_end_event = create_group_timeline_event(
            nonactive_group, i, "E", cell_to_info
        )
        write_timeline_event(group_end_event, out_file)


def create_cell_timeline_event(
    active_cell_info: ActiveCell,
    cycle: int,
    event_type: str,
    cell_to_info: dict[str, TimelineCell],
):
    return {
        "name": active_cell_info.name,
        "cat": "cell",
        "ph": event_type,
        "pid": cell_to_info[active_cell_info.cell_name].pid,
        "tid": 1,
        "ts": cycle * ts_multiplier,
    }


def create_group_timeline_event(
    active_group_info: ActiveEnable,
    cycle: int,
    event_type: str,
    cell_to_info: dict[str, TimelineCell],
):
    cell_info = cell_to_info[active_group_info.cell_name]
    if event_type == "B":
        (pid, tid, name) = cell_info.add_group(active_group_info.enable_name)
    else:
        (pid, tid, name) = cell_info.remove_group(active_group_info.enable_name)
    return {
        "name": name,  # take only the group name for easier visibility
        "cat": "group",
        "ph": event_type,
        "pid": pid,
        "tid": tid,
        "ts": cycle * ts_multiplier,
    }
