from dataclasses import dataclass, field
from functools import reduce
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
class ParentInterval:
    # name: str
    start_cycle: int | None = field(default=None)
    possible_end: int | None = field(default=None)
    active_children: set[str] = field(default_factory=set)
    force_quit_encountered: bool = field(default=False)

    def start_event(self, start_cycle: int, child_track_id: str) -> Optional[tuple[int, int]]:
        # if start_cycle < 50:
        # print(f"START EVENT {start_cycle}. CURRENT VALUES {self.start_cycle} {self.possible_end} {self.active_children}")
        if self.start_cycle is None:
            self.start_cycle = start_cycle
            ret = None
        elif self.possible_end is not None and start_cycle > self.possible_end + 1 and len(self.active_children) == 0:
            prev_start = self.start_cycle
            prev_end = self.possible_end
            self.start_cycle = start_cycle
            self.possible_end = None
            ret = (prev_start, prev_end)
        else:
            # ignore this "start" since it's right next to the previous start.
            # extend possible_end by turning it to None, since we started a new substatement
            self.possible_end = None
            ret = None

        self.active_children.add(child_track_id)
        return ret
    
    def end_event(self, end_cycle: int, child_track_id: str, force_quit: bool = False) -> Optional[tuple[int, int]]:
        """
        If force_quit is True, then we will return the tuple (end of the program).
        """
        # if end_cycle < 50:
        # print(f"END EVENT {end_cycle}. CURRENT VALUES {self.start_cycle} {self.possible_end} {self.active_children}")
        self.active_children.remove(child_track_id)
        if force_quit:
            return (self.start_cycle, end_cycle)
        else:
            # keep extending the possible end
            self.possible_end = end_cycle


    def event(self, cycle: int, event_type: TrackEvent.Type, child_track_id: str, force_quit: bool=False) -> Optional[tuple[int, int]]:
        if force_quit:
            print(f"encountered force quit from child {child_track_id}")
            self.force_quit_encountered = True
        match event_type:
            case TrackEvent.TYPE_SLICE_BEGIN:
                return self.start_event(cycle, child_track_id)
            case TrackEvent.TYPE_SLICE_END:
                return self.end_event(cycle, child_track_id, force_quit)
            
    def force_quit(self):
        if self.force_quit_encountered:
            raise ProfilerException("Force quitting an interval that has already been force quit!")
        self.force_quit_encountered = True
        return (self.start_cycle, self.possible_end)

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

    # track_id --> track id of ancestors (closer parents first)
    track_to_parent_tracks: dict[str, list[str]]
    # parent track id --> interval object
    parent_to_interval: dict[str, ParentInterval]

    def __init__(self, adl_map: AdlMap, dahlia_parent_map: str | None, primitive_metadata: PrimitiveMetadata):
        self.proto = ProtoTimelineWrapper()
        self.primitive_name_to_type = {}
        self.proto.add_collection(self.main_function_name)
        self.proto.add_collection(self.primitive_collection_name)
        if dahlia_parent_map is not None:
            self._process_dahlia_parent_map(adl_map, dahlia_parent_map)
        else:
            print(
                "dahlia_parent_map was not given; somewhat inconvenient timeline view will be generated"
            )

        # FIXME: hella defunct way of creating a lookup for primitives
        for _, p_map in primitive_metadata.p_map.items():
            self.primitive_name_to_type.update(p_map)

    def _parent_block(self, line_contents):
        return f"{self.parent_prefix}{line_contents}"
    
    def _read_json_parent_map(self, parent_map_file):
        """
        JSON is annoying and requires string keys. This function returns a map obtained from parent_map_file, but with int keys instead.
        """
        m = json.load(open(parent_map_file))
        return {int(k) : m[k] for k in m}

    def _process_dahlia_parent_map(self, adl_map: AdlMap, dahlia_parent_map: str):
        """
        Assumes that dahlia_parent_map points to an actual string path.
        """
        self.track_to_parent_tracks = {}
        self.parent_to_interval = {}

        json_parent_map: dict[int, list[int]] = self._read_json_parent_map(dahlia_parent_map)

        # need to have a parent block version of each one
        all_parent_lines: set[int] = reduce((lambda l1, l2: set(l1).union(set(l2))), json_parent_map.values())
        print(f"ALL PARENTS: {all_parent_lines}")

        # figure out child-parent mappings.
        for linum in sorted(json_parent_map, key=(lambda x: len(json_parent_map[x]))):
            line_contents = adl_map.adl_linum_map[linum]

            # identify the immediate ancestor trackids
            if linum in all_parent_lines and len(json_parent_map[linum]) == 0:
                # this line is a parent line with no parents of its own,
                # the parent is the block version of this line.
                parent_track_id = self._parent_block(line_contents)
                self.track_to_parent_tracks[line_contents] = [parent_track_id]

            elif linum in all_parent_lines:
                # this line is a parent line that itself has parents
                block_track_id = self._parent_block(line_contents)
                ancestor_list = list(map((lambda p: self._parent_block(adl_map.adl_linum_map[p])), json_parent_map[linum]))

                # this line's parent is the block version of this line.
                self.track_to_parent_tracks[line_contents] = [block_track_id] + ancestor_list

                # the block version of this line's ancestors are the block version of the actual ancestors of this line.
                self.track_to_parent_tracks[block_track_id] = ancestor_list

            elif len(json_parent_map[linum]) > 0:
                # this line is a "normal" line with ancestors.
                # use block version of the actual ancestors.
                ancestor_list = list(map((lambda p: self._parent_block(adl_map.adl_linum_map[p])), json_parent_map[linum]))

                self.track_to_parent_tracks[line_contents] = ancestor_list

            # otherwise is a "normal" line with NO parents.

        for t in self.track_to_parent_tracks:
            print(f"{t}: \t {self.track_to_parent_tracks[t]}")
        track_ids_covered = set()

        # create tracks and parent interval objects for each parent track.
        for parent_linum in sorted(all_parent_lines, key=(lambda x: len(json_parent_map[x]))):
            block_track_id = self._parent_block(adl_map.adl_linum_map[parent_linum])
            # does the block itself have a parent?
            if block_track_id in self.track_to_parent_tracks:
                # first entry is the immediate parent.
                block_parent_track_id = self.track_to_parent_tracks[block_track_id][0]
            else:
                block_parent_track_id = None
            self.proto.register_track_in_collection(self.main_function_name, block_track_id, intermediate_parent_name=block_parent_track_id)
            self.parent_to_interval[block_track_id] = ParentInterval()
            track_ids_covered.add(block_track_id)

        # create tracks for all non-parent tracks.
        for track_id in set(self.track_to_parent_tracks.keys()).difference(track_ids_covered):
            # first entry is the immediate parent.
            parent_track_id = self.track_to_parent_tracks[track_id][0]
            self.proto.register_track_in_collection(self.main_function_name, track_id, intermediate_parent_name=parent_track_id)


        # # create special tracks and interval object for all lines that are parent
        # for parent_line in all_parent_lines:
        #     line_contents = adl_map.adl_linum_map[parent_line]
        #     # register parent track
        #     parent_track_id = self._parent_block(line_contents)
        #     self.proto.register_track_in_collection(self.main_function_name, parent_track_id, None)
        #     # create interval object and add
        #     self.parent_to_interval[parent_track_id] = ParentInterval()

        # # create tracks for all entries
        # for linum_str in sorted(
        #     json_parent_map, key=(lambda x: len(json_parent_map[x]))
        # ):
        #     linum = int(linum_str)
        #     line_contents = adl_map.adl_linum_map[linum]
        #     # determine the immediate parent
        #     if len(json_parent_map[linum_str]) == 0:
        #         if linum in all_parent_lines:
        #             # this line is the overhead; parent is a block version of itself.
        #             parent_trackid = self._parent_block(line_contents)
        #             self.track_to_parent_track[line_contents] = parent_trackid
        #         else:
        #             parent_trackid = None
        #     else:
        #         parent_line_contents = adl_map.adl_linum_map[json_parent_map[linum_str][0]]
        #         parent_trackid = self._parent_block(parent_line_contents)
        #         self.track_to_parent_track[line_contents] = parent_trackid
        #     self.proto.register_track_in_collection(
        #         self.main_function_name, line_contents, intermediate_parent_name=parent_trackid
        #     )

    def register_statement_event(
        self, statement: str, timestamp: int, event_type: TrackEvent.Type, force_quit: bool=False
    ):
        # print(f"registering statement {statement}. {statement in self.track_to_parent_track}")
        if not self.proto.is_track_registered_in_collection(
            self.main_function_name, statement
        ):
            self.proto.register_track_in_collection(self.main_function_name, statement)
        self.proto.register_event_in_collection(
            self.main_function_name, statement, statement, timestamp, event_type
        )

        # if this statement is a child of a parent block, 
        if statement in self.track_to_parent_tracks:
            # print(statement)
            for parent_track_id in self.track_to_parent_tracks[statement]:
                # if "0025" in parent_track_id:
                #     print()
                interval = self.parent_to_interval[parent_track_id]
                # if timestamp < 50:
                    # print(f"PARENT {parent_track_id}")
                res_opt = interval.event(timestamp, event_type, statement, force_quit)
                if res_opt is not None:
                    # add previous start and end interval events.
                    (parent_start, parent_end) = res_opt
                    # if timestamp < 50:
                        # print(f"SUCCESS PARENT {parent_track_id} START: {parent_start} PARENT END: {parent_end}")
                    self.proto.register_event_in_collection(self.main_function_name, parent_track_id, parent_track_id, parent_start, TrackEvent.TYPE_SLICE_BEGIN)
                    self.proto.register_event_in_collection(self.main_function_name, parent_track_id, parent_track_id, parent_end, TrackEvent.TYPE_SLICE_END)


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

    def _force_quit_all_parent_intervals(self):
        for block_id in self.parent_to_interval:
            print(block_id)
            interval = self.parent_to_interval[block_id]
            print(interval)
            if not interval.force_quit_encountered:
                (block_start, block_end) = interval.force_quit()
                print(f"FORCE QUIT BLOCK {block_id}, {block_start} {block_end}")
                self.proto.register_event_in_collection(self.main_function_name, block_id, block_id, block_start, TrackEvent.TYPE_SLICE_BEGIN)
                self.proto.register_event_in_collection(self.main_function_name, block_id, block_id, block_end, TrackEvent.TYPE_SLICE_END)


    def emit(self, out_path: str):
        print("hello????")
        # FIXME: maybe not put this here?
        self._force_quit_all_parent_intervals()
        self.proto.emit(out_path)
