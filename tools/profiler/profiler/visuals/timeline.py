from dataclasses import dataclass
import os

import uuid

from perfetto.trace_builder.proto_builder import TraceProtoBuilder
from perfetto.protos.perfetto.trace.perfetto_trace_pb2 import (
    TrackEvent,
)


from profiler.classes.tracedata import (
    TraceData,
    ControlRegUpdates,
    StackElementType,
)
from profiler.classes.cell_metadata import CellMetadata

ts_multiplier = 1  # [timeline view] ms on perfetto UI that resembles a single cycle

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
