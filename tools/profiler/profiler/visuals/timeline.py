import os
from functools import reduce
import json

from typing import Optional

from perfetto.protos.perfetto.trace.perfetto_trace_pb2 import (
    TrackEvent,
)

from profiler.classes.adl import AdlMap
from profiler.classes.cell_metadata import CellMetadata
from profiler.classes.primitive_metadata import PrimitiveMetadata
from profiler.classes.tracedata import CycleTrace, TraceData, StackElementType, PTrace
from profiler.classes.visuals.timeline import (
    CalyxProtoTimeline,
    DahliaProtoTimeline,
    BlockInterval,
    block_name,
)


def compute_calyx_protobuf_timeline(
    tracedata: TraceData,
    cell_metadata: CellMetadata,
    primitive_metadata: PrimitiveMetadata,
    enable_thread_data: dict[str, dict[str, int]],
    out_dir: str,
):
    calyx_proto: CalyxProtoTimeline = CalyxProtoTimeline(
        enable_thread_data, cell_metadata, tracedata, primitive_metadata
    )

    currently_active_cells: set[str] = set()
    currently_active_ctrl_groups: set[str] = set()
    currently_active_groups: set[str] = set()
    currently_active_primitives: set[str] = set()

    for i in tracedata.trace_with_control_groups:
        print(i)
        this_cycle_active_ctrl_groups: set[str] = set()
        this_cycle_active_cells: set[str] = set()
        this_cycle_active_groups: set[str] = set()
        this_cycle_active_primitives: set[str] = set()
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
                    case StackElementType.PRIMITIVE:
                        this_cycle_active_primitives.add(
                            f"{stack_acc}.{stack_elem.name}"
                        )

        # cells

        for done_cell in currently_active_cells.difference(this_cycle_active_cells):
            calyx_proto.register_cell_event(done_cell, i, TrackEvent.TYPE_SLICE_END)

        for new_cell in this_cycle_active_cells.difference(currently_active_cells):
            calyx_proto.register_cell_event(new_cell, i, TrackEvent.TYPE_SLICE_BEGIN)

        # control groups

        for gone_ctrl_group in sorted(
            currently_active_ctrl_groups.difference(this_cycle_active_ctrl_groups)
        ):
            calyx_proto.register_control_event(
                gone_ctrl_group, i, TrackEvent.TYPE_SLICE_END
            )

        for new_ctrl_group in sorted(
            this_cycle_active_ctrl_groups.difference(currently_active_ctrl_groups)
        ):
            calyx_proto.register_control_event(
                new_ctrl_group, i, TrackEvent.TYPE_SLICE_BEGIN
            )

        # normal groups

        for done_group in currently_active_groups.difference(this_cycle_active_groups):
            calyx_proto.register_enable_event(done_group, i, TrackEvent.TYPE_SLICE_END)

        for new_group in this_cycle_active_groups.difference(currently_active_groups):
            calyx_proto.register_enable_event(new_group, i, TrackEvent.TYPE_SLICE_BEGIN)

        # primitives

        for done_primitive in currently_active_primitives.difference(
            this_cycle_active_primitives
        ):
            print(f"Ending primitive {done_primitive}")
            calyx_proto._register_primitive_event(
                done_primitive, i, TrackEvent.TYPE_SLICE_END
            )
        for new_primitive in this_cycle_active_primitives.difference(
            currently_active_primitives
        ):
            print(f"Starting primitive {new_primitive}")
            calyx_proto._register_primitive_event(
                new_primitive, i, TrackEvent.TYPE_SLICE_BEGIN
            )

        # update
        currently_active_cells = this_cycle_active_cells
        currently_active_ctrl_groups = this_cycle_active_ctrl_groups
        currently_active_groups = this_cycle_active_groups
        currently_active_primitives = this_cycle_active_primitives

    # elements that are active until the very end

    for active_at_end_cell in currently_active_cells:
        calyx_proto.register_cell_event(
            active_at_end_cell, i + 1, TrackEvent.TYPE_SLICE_END
        )

    for active_at_end_ctrl_group in currently_active_ctrl_groups:
        calyx_proto.register_control_event(
            active_at_end_ctrl_group, i + 1, TrackEvent.TYPE_SLICE_END
        )

    for active_at_end_group in currently_active_groups:
        calyx_proto.register_enable_event(
            active_at_end_group, i + 1, TrackEvent.TYPE_SLICE_END
        )

    for active_at_end_primitive in currently_active_primitives:
        calyx_proto._register_primitive_event(
            active_at_end_primitive, i + 1, TrackEvent.TYPE_SLICE_END
        )

    out_path = os.path.join(out_dir, "timeline_trace.pftrace")
    calyx_proto.emit(out_path)


def _read_json_parent_map(parent_map_file):
    """
    Helper function for _process_dahlia_parent_map()
    JSON is annoying and requires string keys. This function returns a map obtained from parent_map_file, but with int keys instead.
    """
    m = json.load(open(parent_map_file))
    return {int(k): m[k] for k in m}


def _process_dahlia_parent_map(adl_map: AdlMap, dahlia_block_map: str | None):
    """
    Helper function for compute_dahlia_protobuf_timeline()
    """
    statement_to_block_ancestors: dict[str, list[str]] = {}
    if dahlia_block_map is None:
        # return default dictionary (every line has zero block ancestors) and set.
        print(
            "dahlia_parent_map was not given; somewhat inconvenient timeline view will be generated"
        )
        return {
            adl_map.adl_linum_map[linum]: [] for linum in adl_map.adl_linum_map
        }, set()

    json_parent_map: dict[int, list[int]] = _read_json_parent_map(dahlia_block_map)

    # need to have a parent block version of each one
    all_block_linums: set[int] = reduce(
        (lambda l1, l2: set(l1).union(set(l2))), json_parent_map.values()
    )

    linum_to_block = {
        linum: block_name(adl_map.adl_linum_map[linum]) for linum in all_block_linums
    }

    # figure out child-parent mappings.
    for linum in sorted(json_parent_map, key=(lambda x: len(json_parent_map[x]))):
        line_contents = adl_map.adl_linum_map[linum]

        # identify the immediate ancestor trackids
        if linum in all_block_linums and len(json_parent_map[linum]) == 0:
            # this line is a parent line with no parents of its own,
            # the parent is the block version of this line.
            block_track_id = block_name(line_contents)
            statement_to_block_ancestors[line_contents] = [block_track_id]
            # block version also gets added?
            statement_to_block_ancestors[block_track_id] = []

        elif linum in all_block_linums:
            # this line is a parent line that itself has parents
            block_track_id = block_name(line_contents)
            ancestor_list = list(
                map((lambda p: linum_to_block[p]), json_parent_map[linum])
            )

            # this line's parent is the block version of this line.
            statement_to_block_ancestors[line_contents] = [
                block_track_id
            ] + ancestor_list

            # the block version of this line's ancestors are the block version of the actual ancestors of this line.
            statement_to_block_ancestors[block_track_id] = ancestor_list

        elif len(json_parent_map[linum]) > 0:
            # this line is a "normal" line with ancestors.
            # use block version of the actual ancestors.
            ancestor_list = list(
                map(
                    (lambda p: block_name(adl_map.adl_linum_map[p])),
                    json_parent_map[linum],
                )
            )

            statement_to_block_ancestors[line_contents] = ancestor_list

        else:
            # otherwise is a "normal" line with NO parents.
            statement_to_block_ancestors[line_contents] = []

    return statement_to_block_ancestors, set(linum_to_block.values())


def compute_dahlia_protobuf_timeline(
    adl_map: AdlMap,
    dahlia_trace: PTrace,
    dahlia_parent_map: str | None,
    out_dir: str,
    calyx_trace: PTrace,
    primitive_metadata: PrimitiveMetadata,
):
    dahlia_proto: DahliaProtoTimeline = DahliaProtoTimeline(
        adl_map, dahlia_parent_map, primitive_metadata
    )
    # statement --> [b1, b2, ...] where b1 is the immediate parent
    stmt_to_block_ancestors: dict[str, list[str]]
    # names of all blocks
    blocks: set[str]
    # figure out child-ancestor relationships
    stmt_to_block_ancestors, blocks = _process_dahlia_parent_map(
        adl_map, dahlia_parent_map
    )
    # construct blocks with this knowledge
    dahlia_proto.create_tracks(stmt_to_block_ancestors, blocks)

    currently_active_statements: set[str] = set()
    # if a block is not in the dictionary, it means it;s not currently active.
    currently_active_blocks: dict[str, BlockInterval] = {}

    for i in dahlia_trace:
        # blocks should get a "done" event when they get zero active statements and no "starts" this cycle.
        blocks_ended_this_cycle: set[str] = set()
        statements_active_this_cycle: set[str] = set()
        i_trace: CycleTrace = dahlia_trace[i]
        for stacks in i_trace.stacks:
            for statement in stacks:
                statements_active_this_cycle.add(statement.name)

        # statements that ended
        for done_statement in currently_active_statements.difference(
            statements_active_this_cycle
        ):
            dahlia_proto.register_statement_event(
                done_statement, i, TrackEvent.TYPE_SLICE_END
            )
            # for each block of the stmt, signal that the stmt has ended
            for block in stmt_to_block_ancestors[done_statement]:
                block_interval = currently_active_blocks[block]
                block_interval.stmt_end(i, done_statement)
                if block_interval.num_active_children() == 0:
                    blocks_ended_this_cycle.add(block)

        # statements that started
        for started_statement in statements_active_this_cycle.difference(
            currently_active_statements
        ):
            dahlia_proto.register_statement_event(
                started_statement, i, TrackEvent.TYPE_SLICE_BEGIN
            )
            # for each block of the stmt, signal that the stmt has begun
            for block in stmt_to_block_ancestors[started_statement]:
                if block not in currently_active_blocks:
                    # create new interval and record a start event to the timeline.
                    block_interval = BlockInterval(i)
                    currently_active_blocks[block] = block_interval
                    dahlia_proto.register_statement_event(
                        block, block_interval.start_cycle, TrackEvent.TYPE_SLICE_BEGIN
                    )
                else:
                    block_interval = currently_active_blocks[block]
                block_interval.stmt_start_event(started_statement)
                # NOTE: need to remove any blocks that were already in blocks_ended_this_cycle since we observed a start stmt in the same cycle that we thought the block was done.
                if block in blocks_ended_this_cycle:
                    blocks_ended_this_cycle.remove(block)

        # check for blocks that ended this cycle.
        for block in blocks_ended_this_cycle:
            block_interval = currently_active_blocks[block]
            dahlia_proto.register_statement_event(block, i, TrackEvent.TYPE_SLICE_END)
            del currently_active_blocks[block]

        currently_active_statements = statements_active_this_cycle

    # we processed the whole trace.
    # Add end events for all active statements.
    for active_at_end_statement in currently_active_statements:
        dahlia_proto.register_statement_event(
            active_at_end_statement, i + 1, TrackEvent.TYPE_SLICE_END
        )

    # Add end events for all active blocks.
    for active_at_end_block in currently_active_blocks:
        dahlia_proto.register_statement_event(
            active_at_end_block, i + 1, TrackEvent.TYPE_SLICE_END
        )

    # PASS 2 FOR PRIMITIVES

    # scan through Calyx trace to see what primitives were active.
    # there is probably a more efficient way to do this
    # FIXME: just going to state the primitive name for now. Probably want fully qualified
    current_active_primitives: set[str] = set()
    for i in calyx_trace:
        primitives_active_this_cycle: set[str] = set()
        for stack in calyx_trace[i].stacks:
            for stack_elem in stack:
                match stack_elem.element_type:
                    case StackElementType.PRIMITIVE:
                        primitives_active_this_cycle.add(stack_elem.name)

        for done_primitive in current_active_primitives.difference(
            primitives_active_this_cycle
        ):
            dahlia_proto.register_calyx_primitive_event(
                done_primitive, i, TrackEvent.TYPE_SLICE_END
            )
        for new_primitive in primitives_active_this_cycle.difference(
            current_active_primitives
        ):
            dahlia_proto.register_calyx_primitive_event(
                new_primitive, i, TrackEvent.TYPE_SLICE_BEGIN
            )

        current_active_primitives = primitives_active_this_cycle

    for active_at_end_primitive in current_active_primitives:
        dahlia_proto.register_calyx_primitive_event(
            active_at_end_primitive, i + 1, TrackEvent.TYPE_SLICE_END
        )

    out_path = os.path.join(out_dir, "dahlia_timeline_trace.pftrace")
    dahlia_proto.emit(out_path)
