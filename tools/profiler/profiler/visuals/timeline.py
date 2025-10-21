import os


from perfetto.protos.perfetto.trace.perfetto_trace_pb2 import (
    TrackEvent,
)

from profiler.classes.adl import AdlMap
from profiler.classes.cell_metadata import CellMetadata
from profiler.classes.primitive_metadata import PrimitiveMetadata
from profiler.classes.tracedata import CycleTrace, TraceData, StackElementType, PTrace
from profiler.classes.visuals.timeline import CalyxProtoTimeline, DahliaProtoTimeline


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
            calyx_proto._register_primitive_event(
                done_primitive, i, TrackEvent.TYPE_SLICE_END
            )
        for new_primitive in this_cycle_active_primitives.difference(
            currently_active_primitives
        ):
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

    out_path = os.path.join(out_dir, "timeline_trace.pftrace")
    calyx_proto.emit(out_path)


def compute_adl_protobuf_timeline(
    adl_map: AdlMap,
    dahlia_trace: PTrace,
    dahlia_parent_map: str | None,
    out_dir: str,
    calyx_trace: PTrace,
    primitive_metadata: PrimitiveMetadata,
):
    dahlia_proto: DahliaProtoTimeline = DahliaProtoTimeline(adl_map, dahlia_parent_map, primitive_metadata)

    currently_active_statements: set[str] = set()

    for i in dahlia_trace:
        if i < 50:
            print(f"=============================CYCLE!!!! {i}")
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

        # statements that started
        for started_statement in statements_active_this_cycle.difference(
            currently_active_statements
        ):
            dahlia_proto.register_statement_event(
                started_statement, i, TrackEvent.TYPE_SLICE_BEGIN
            )

        currently_active_statements = statements_active_this_cycle

    for active_at_end_statement in currently_active_statements:
        dahlia_proto.register_statement_event(
            active_at_end_statement, i + 1, TrackEvent.TYPE_SLICE_END, True
        )

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
