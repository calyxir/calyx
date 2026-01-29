import os

from profiler.visuals import flame, timeline
from profiler.classes.primitive_metadata import PrimitiveMetadata
from profiler.classes.adl import AdlMap, Adl, DahliaAdlMap, SourceLoc
from profiler.classes.tracedata import FlameMapMode, TraceData, PTrace, CycleTrace
from .classes.stack_element import StackElement, StackElementType


def create_dahlia_trace(tracedata: TraceData, dahlia_map: DahliaAdlMap):
    # AYAKA TODO: incorporate block information so we can generate a nice Flame graph with nesting.
    calyx_trace: PTrace = tracedata.trace_with_control_groups
    dahlia_trace: PTrace = PTrace()
    groups_no_mapping: set[str] = set()
    for i in calyx_trace:
        # find leaf groups (there could be some in parallel)
        i_trace: CycleTrace = calyx_trace[i]
        leaf_groups: set = i_trace.find_leaf_groups()
        # FIXME: hardcoding to main since Dahlia programs rarely have multiple components.
        group_map = dahlia_map.group_map.get("main")
        # Dahlia StackElements that are active this cycle
        dahlia_stacks: list[list[StackElement]] = []
        # contents of stack elements that are active on a "thread"
        # will use `map` to convert each element into a StackElement
        raw_stack_items: list[str]
        for group in leaf_groups:
            if group not in group_map:
                groups_no_mapping.add(group)
                raw_stack_items = f"CALYX: '{group}'"
            else:
                group_sourceloc: SourceLoc = group_map[group]
                entry = dahlia_map.adl_linum_map[group_sourceloc.linenum]
                raw_stack_items = (
                    dahlia_map.stmt_to_block_ancestors[entry]
                    if entry in dahlia_map.stmt_to_block_ancestors
                    else []
                )
                raw_stack_items.append(entry)
            dahlia_group = StackElement(
                entry, StackElementType.ADL_LINE, sourceloc=group_sourceloc
            )
            # grab any ancestors and create stack

            dahlia_stacks.append([dahlia_group])
        dahlia_trace.add_cycle(i, CycleTrace(dahlia_stacks))
    print(f"\tGroups without ADL mapping: {groups_no_mapping}")
    return dahlia_trace


def create_and_write_adl_map(
    tracedata: TraceData,
    primitive_metadata: PrimitiveMetadata,
    adl_mapping_file: str,
    out_dir: str,
    dahlia_parent_map: str | None = None,
):
    """
    Creates ADL and Mixed (ADL + Calyx; where applicable) versions of flame graph maps.
    """
    print(f"Creating ADL visuals from adl_map: {adl_mapping_file}")

    adl_flat_flame_file = os.path.join(out_dir, "adl-flat-flame.folded")
    adl_scaled_flame_file = os.path.join(out_dir, "adl-scaled-flame.folded")
    mixed_flat_flame_file = os.path.join(out_dir, "mixed-flat-flame.folded")
    mixed_scaled_flame_file = os.path.join(out_dir, "mixed-scaled-flame.folded")
    adl_map = AdlMap(adl_mapping_file)

    match adl_map.adl:
        case Adl.DAHLIA:
            # add Dahlia-specific map info (block and statement hierarchy)
            dahlia_map = DahliaAdlMap(adl_map, dahlia_parent_map)
            print(dahlia_map.stmt_to_block_ancestors)
            # We will create a Dahlia-specific trace
            dahlia_trace = create_dahlia_trace(tracedata, dahlia_map)
            print("DAHLIA TRACE!!!!!")
            for d in dahlia_trace.trace:
                print(d)
            # flame.create_and_write_dahlia_flame_maps(
            #     tracedata, adl_mapping_file, out_dir
            # )

            # timeline.compute_dahlia_protobuf_timeline(
            #     dahlia_map,
            #     dahlia_trace,
            #     out_dir,
            #     tracedata.trace,
            #     primitive_metadata,
            # )

        case Adl.PY:
            # for Calyx-py we can suffice with just using Calyx PTraces
            adl_added_trace = tracedata.add_sourceloc_info(adl_map)

            flame.create_and_write_calyx_flame_maps(
                adl_added_trace,
                out_dir,
                adl_flat_flame_file,
                scaled_flame_out=adl_scaled_flame_file,
                mode=FlameMapMode.ADL,
            )
            flame.create_and_write_calyx_flame_maps(
                adl_added_trace,
                out_dir,
                mixed_flat_flame_file,
                scaled_flame_out=mixed_scaled_flame_file,
                mode=FlameMapMode.MIXED,
            )
