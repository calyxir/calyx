from dataclasses import dataclass, field
from typing import Optional
import uuid
import json


from perfetto.trace_builder.proto_builder import TraceProtoBuilder
from perfetto.protos.perfetto.trace.perfetto_trace_pb2 import (
    TrackEvent,
)

from profiler.classes.adl import AdlMap
from profiler.classes.primitive_metadata import PrimitiveMetadata
from profiler.classes.errors import ProfilerException
from profiler.classes.tracedata import TraceData
from profiler.classes.cell_metadata import CellMetadata


@dataclass
class ProtoTimelineCollection:
    """
    Generalized class for creating . A "Collection" is a track (ex. a cell in Calyx) from which there will be child classes.
    A collection can have "intermediate tracks", which are children of the "Collection" but could have children of their own.
    Abstracts the process of obtaining uuids for specific tracks.
    """

    builder: TraceProtoBuilder
    collection_uuid: int
    # sid for the entire colelction
    sid_val: int
    # drop down for control groups
    # intermediate_tracks: dict[str, int]
    track_name_to_uuid: dict[str, int]

    def __init__(
        self,
        builder: TraceProtoBuilder,
        collection_name: str,
        sid: int,
        intermediate_track_names: set[str],
    ):
        self.builder = builder
        self.sid_val = sid
        self.collection_uuid = self._define_track(collection_name)
        self.track_name_to_uuid = {collection_name: self.collection_uuid}
        # create intermediate tracks (ex. "Control Groups"), which have their own dropdown
        # self.intermediate_tracks = {}
        for track_name in intermediate_track_names:
            self.track_name_to_uuid[track_name] = self._define_track(
                track_name, parent_track_uuid=self.collection_uuid
            )

    # Helper to define a new track with a unique UUID
    def _define_track(self, track_name, parent_track_uuid=None):
        track_uuid = uuid.uuid4().int & ((1 << 63) - 1)
        packet = self.builder.add_packet()
        packet.track_descriptor.uuid = track_uuid
        packet.track_descriptor.name = track_name  # self.fully_qualified_cell_name
        if parent_track_uuid:
            packet.track_descriptor.parent_uuid = parent_track_uuid

        return track_uuid

    # Helper to add an event to a track
    def _add_slice_event(self, ts, event_type, event_track_uuid, name):
        packet = self.builder.add_packet()
        packet.timestamp = ts
        packet.track_event.type = event_type
        packet.track_event.track_uuid = event_track_uuid
        if name:
            packet.track_event.name = name
        packet.trusted_packet_sequence_id = self.sid_val

    def create_new_track(self, track_id, intermediate_parent_name=None):
        if intermediate_parent_name is not None:
            # check intermediate_tracks and see whether there's a match
            # if intermediate_parent_name not in self.intermediate_tracks:
            if intermediate_parent_name not in self.track_name_to_uuid:
                raise ProfilerException("Invalid intermediate parent name!")
            parent_uuid = self.track_name_to_uuid[intermediate_parent_name]
        else:
            parent_uuid = self.collection_uuid
        track_uuid = self._define_track(track_id, parent_track_uuid=parent_uuid)
        self.track_name_to_uuid[track_id] = track_uuid

    def is_track_registered(self, track_id: str):
        ret = track_id in self.track_name_to_uuid
        return ret

    def register_event(
        self,
        event_name: str,
        track_name: str,
        timestamp: int,
        event_type: TrackEvent.Type,
    ):
        if track_name in self.track_name_to_uuid:  # [track_id]
            track_uuid = self.track_name_to_uuid[track_name]
            self._add_slice_event(timestamp, event_type, track_uuid, event_name)
        else:
            raise ProfilerException(
                f'Track "{track_name}" should be registered before its first event!'
            )


@dataclass
class ProtoTimelineWrapper:
    """
    A class representing an *entire* timeline view for a program.
    Outside code should use methods in this class instead of directly calling methods on ProtoTimelineCollection.
    """

    builder: TraceProtoBuilder
    name_to_collection: dict[str, ProtoTimelineCollection]
    sid_acc: int
    default_intermediate_track_names: set[str]

    def __init__(self, default_intermediate_track_names: set[str] | None = None):
        self.builder = TraceProtoBuilder()
        self.name_to_collection = {}
        self.sid_acc = 300
        self.default_intermediate_track_names = (
            default_intermediate_track_names
            if default_intermediate_track_names is not None
            else set()
        )

    def add_collection(
        self, collection_name, intermediate_track_names: set[str] | None = None
    ):
        intermediate_tracks = (
            intermediate_track_names
            if intermediate_track_names is not None
            else self.default_intermediate_track_names
        )
        if collection_name not in self.name_to_collection:
            self.name_to_collection[collection_name] = ProtoTimelineCollection(
                self.builder, collection_name, self.sid_acc, intermediate_tracks
            )
            self.sid_acc += 1

    def is_track_registered_in_collection(self, collection_name: str, track_id: str):
        if collection_name not in self.name_to_collection:
            raise ProfilerException(f"Collection {collection_name} not stored!")
        return self.name_to_collection[collection_name].is_track_registered(track_id)

    def register_track_in_collection(
        self, collection_name: str, track_id: str, intermediate_parent_name=None
    ):
        collection: ProtoTimelineCollection = self.name_to_collection[collection_name]
        collection.create_new_track(track_id, intermediate_parent_name)

    def register_event_in_collection(
        self,
        collection_name: str,
        event_name: str,
        track_id: str,
        timestamp: int,
        event_type: TrackEvent.Type,
    ):
        # events occur in tracks
        collection: ProtoTimelineCollection = self.name_to_collection[collection_name]
        collection.register_event(event_name, track_id, timestamp, event_type)

    def emit(self, output_filename):
        with open(output_filename, "wb") as f:
            f.write(self.builder.serialize())


@dataclass
class CalyxProtoTimeline:
    """
    A class creating a Perfetto timeline in the program structure of
    Calyx programs (cells, control registers, groups).
    """

    proto: ProtoTimelineWrapper
    cell_to_enables_to_track: dict[str, dict[str, str]]
    cell_metadata: CellMetadata
    primitives_metadata: PrimitiveMetadata
    primitives_track_name = "Primitives"
    control_groups_track_name = "Control Groups"
    control_updates_track_name = "Control Register Updates"
    misc_groups_track_name = "Non-id-ed groups"

    def __init__(
        self, enable_thread_data, cell_metadata: CellMetadata, tracedata: TraceData, primitives_metadata: PrimitiveMetadata
    ):
        self.cell_metadata = cell_metadata
        self.primitives_metadata = primitives_metadata

        self.proto = ProtoTimelineWrapper(
            {self.primitives_track_name, self.control_groups_track_name}
        )
        # set up data structures to track cells and groups
        self.cell_to_enables_to_track = {}
        cell_to_tracks: dict[str, set[str]] = {}
        for component in cell_metadata.component_to_cells:
            enable_to_track_num: dict[str, int] = enable_thread_data[component]
            enable_to_track_name: dict[str, str] = {
                enable: f"Thread {enable_to_track_num[enable]:03}"
                for enable in enable_to_track_num
            }
            for cell in cell_metadata.component_to_cells[component]:
                self.cell_to_enables_to_track[cell] = enable_to_track_name
                cell_to_tracks[cell] = set(enable_to_track_name.values())

        # set up starter info for each cell
        for cell in self.cell_to_enables_to_track:
            self.proto.add_collection(cell)
            # enable tracks that are assigned under enable_thread_data
            for track in cell_to_tracks[cell]:
                self.proto.register_track_in_collection(cell, track)
            # track for groups that did not get an assigned threadid
            self.proto.register_track_in_collection(cell, self.misc_groups_track_name)
            # track for control register updates
            self.proto.register_track_in_collection(
                cell, self.control_updates_track_name
            )

            # ALL of the control register updates
            self._port_register_updates(cell, tracedata)

    def _port_register_updates(self, cell_name: str, tracedata: TraceData):
        if cell_name not in tracedata.control_reg_updates:
            # cells that are not in control_updates do not have any control register updates
            # they are probably single-group components.
            return
        for update_info in tracedata.control_reg_updates[cell_name]:
            # start event
            self._register_control_register_event(
                cell_name,
                update_info.updates,
                update_info.clock_cycle,
                TrackEvent.TYPE_SLICE_BEGIN,
            )
            # end event (next cycle)
            self._register_control_register_event(
                cell_name,
                update_info.updates,
                update_info.clock_cycle + 1,
                TrackEvent.TYPE_SLICE_END,
            )

    def register_cell_event(
        self, cell_name: str, timestamp: int, event_type: TrackEvent.Type
    ):
        self.proto.register_event_in_collection(
            cell_name, cell_name, cell_name, timestamp, event_type
        )

    def _register_control_register_event(
        self, cell: str, updates: str, timestamp: int, event_type: TrackEvent.Type
    ):
        """
        Adds a control register update event for the specified cell.
        """
        self.proto.register_event_in_collection(
            cell, updates, self.control_updates_track_name, timestamp, event_type
        )

    def register_enable_event(
        self, fully_qualified_group: str, timestamp: int, event_type: TrackEvent.Type
    ):
        name_split = fully_qualified_group.split(".")
        cell = ".".join(name_split[:-1])
        enable_name = name_split[-1]
        group_name = enable_name.split("UG")[0] if "UG" in enable_name else enable_name

        # identify the thread for which this group enable will be written to
        if enable_name in self.cell_to_enables_to_track[cell]:
            thread_name = self.cell_to_enables_to_track[cell][enable_name]
        else:
            # wasn't assigned a thread, so will go into the misc category
            thread_name = self.misc_groups_track_name

        self.proto.register_event_in_collection(
            cell, group_name, thread_name, timestamp, event_type
        )

    def register_control_event(
        self,
        fully_qualified_ctrl_group: str,
        timestamp: int,
        event_type: TrackEvent.Type,
    ):
        name_split = fully_qualified_ctrl_group.split(".")
        cell = ".".join(name_split[:-1])
        ctrl_group_str = name_split[-1]
        # use source locations when available. FIXME: make this less hacky
        if "~ " in ctrl_group_str:
            ctrl_group = ctrl_group_str.split("~ ")[1].split("(")[0]
        else:
            ctrl_group = ctrl_group_str.split("(")[0]
        track_id = f"Control Group: {ctrl_group}"

        # add control group track if it's not registered
        if not self.proto.is_track_registered_in_collection(cell, track_id):
            self.proto.register_track_in_collection(
                cell, track_id, intermediate_parent_name=self.control_groups_track_name
            )

        self.proto.register_event_in_collection(
            cell, ctrl_group, track_id, timestamp, event_type
        )

    def _register_primitive_event(
        self,
        fully_qualified_primitive: str,
        timestamp: int,
        event_type: TrackEvent.Type,
    ):
        name_split = fully_qualified_primitive.split(".")
        cell = ".".join(name_split[:-1])
        primitive_name = name_split[-1]


        # Parent track: primitive type
        component = self.cell_metadata.get_component_of_cell(cell)
        primitive_type = self.primitives_metadata.obtain_entry(component, primitive_name)
        # parent intermediate track is the primitive type
        if not self.proto.is_track_registered_in_collection(cell, primitive_type):
            self.proto.register_track_in_collection(cell, primitive_type, intermediate_parent_name=self.primitives_track_name)

        # FIXME: track id contains both the primitive name and type
        track_id = f"{primitive_name} [{primitive_type}]"

        if not self.proto.is_track_registered_in_collection(cell, primitive_name):
            self.proto.register_track_in_collection(
                cell, track_id, intermediate_parent_name=primitive_type
            )

        self.proto.register_event_in_collection(
            cell, primitive_name, track_id, timestamp, event_type
        )

    def emit(self, out_path: str):
        self.proto.emit(out_path)


@dataclass
class BlockInterval:
    # name: str
    start_cycle: int
    possible_end: int | None = field(default=None)
    active_children: set[str] = field(default_factory=set)

    def __init__(self, cycle: int):
        self.start_cycle = cycle
        self.active_children = set()

    def stmt_start_event(self, stmt: str):
        self.active_children.add(stmt)

    def num_active_children(self):
        return len(self.active_children)
    
    def stmt_start(self, stmt_track_id: str):
        assert(stmt_track_id not in self.active_children)
        self.active_children.add(stmt_track_id)

    def stmt_end(self, end_cycle: int, stmt_track_id: str):
        self.possible_end = end_cycle
        # print(f"INTERVAL END EVENT {stmt_track_id} {self.active_children}")
        assert(stmt_track_id in self.active_children)
        self.active_children.remove(stmt_track_id)

def block_name(line_contents: str):
    block_prefix = "B"
    return f"{block_prefix}{line_contents}"

class DahliaProtoTimeline:
    """
    A class creating a Perfetto timeline in the program structure of
    Dahlia programs (statements).
    Contains an extra collection for showing Calyx primitive activity.
    """

    proto: ProtoTimelineWrapper
    primitive_name_to_type: dict[str, str] = field(default_factory=dict)
    # # dahlia line # --> dahlia line # of immediate parent
    # parent_map: dict[int, list[int]] = field(default_factory=dict)
    main_function_name = "main"
    primitive_collection_name = "Calyx Primitives"
    parent_prefix = "B"

    def __init__(self, adl_map: AdlMap, dahlia_parent_map: str | None, primitive_metadata: PrimitiveMetadata):
        self.proto = ProtoTimelineWrapper()
        self.primitive_name_to_type = {}
        self.proto.add_collection(self.main_function_name)
        self.proto.add_collection(self.primitive_collection_name)

        # FIXME: hella defunct way of creating a lookup for primitives
        for _, p_map in primitive_metadata.p_map.items():
            self.primitive_name_to_type.update(p_map)

    def create_tracks(self, statements_to_block_ancestors: dict[str, list[str]], blocks: set[str]):
        # create tracks for each block
        # list needs to be sorted because Protobuf will error out if we assign a nonexistend parent
        for block in sorted(blocks, key=(lambda x: len(statements_to_block_ancestors[x]))):
            block_ancestors = statements_to_block_ancestors[block]
            parent_track_id = block_ancestors[0] if len(block_ancestors) > 0 else None
            self.proto.register_track_in_collection(self.main_function_name, block, intermediate_parent_name=parent_track_id)

        # create tracks for each statement
        for stmt in set(statements_to_block_ancestors.keys()).difference(blocks):
            stmt_ancestors = statements_to_block_ancestors[stmt]
            parent_track_id = stmt_ancestors[0] if len(stmt_ancestors) > 0 else None
            self.proto.register_track_in_collection(self.main_function_name, stmt, intermediate_parent_name=parent_track_id)

    def register_statement_event(
        self, statement: str, timestamp: int, event_type: TrackEvent.Type
    ):
        # print(f"registering statement {statement}. {statement in self.track_to_parent_track}")
        if not self.proto.is_track_registered_in_collection(
            self.main_function_name, statement
        ):
            self.proto.register_track_in_collection(self.main_function_name, statement)
        self.proto.register_event_in_collection(
            self.main_function_name, statement, statement, timestamp, event_type
        )


    def register_calyx_primitive_event(
        self, primitive: str, timestamp: int, event_type: TrackEvent.Type
    ):
        # currently assumes that there are no duplicate cell names, which is quite dangerous. Need to fix
        primitive_type = self.primitive_name_to_type[primitive]
        if not self.proto.is_track_registered_in_collection(self.primitive_collection_name, primitive_type):
            self.proto.register_track_in_collection(
                self.primitive_collection_name, primitive_type
            )

        track_id = f"{primitive} [{primitive_type}]"

        if not self.proto.is_track_registered_in_collection(
            self.primitive_collection_name, track_id
        ):
            self.proto.register_track_in_collection(
                self.primitive_collection_name, track_id, intermediate_parent_name=primitive_type
            )
        self.proto.register_event_in_collection(
            self.primitive_collection_name, primitive, track_id, timestamp, event_type
        )

    def emit(self, out_path: str):
        self.proto.emit(out_path)
