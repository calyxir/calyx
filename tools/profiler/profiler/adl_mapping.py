import os

from profiler.visuals import flame, timeline
from profiler.classes.adl import AdlMap, Adl
from profiler.classes.tracedata import FlameMapMode, TraceData, PTrace, CycleTrace
from .classes.stack_element import StackElement, StackElementType

def create_dahlia_trace(tracedata: TraceData, adl_map: AdlMap):
    calyx_trace: PTrace = tracedata.trace_with_control_groups
    dahlia_trace: PTrace = PTrace()
    for i in calyx_trace:
        # find leaf groups (there could be some in parallel)
        i_trace: CycleTrace = calyx_trace[i]
        leaf_groups: set = i_trace.find_leaf_groups()
        # FIXME: hardcoding to main right now.
        group_map = adl_map.group_map.get("main")
        dahlia_stacks: list[list[StackElement]] = []
        for group in leaf_groups:
            entry = group_map[group].adl_str()
            dahlia_group = StackElement(entry, StackElementType.ADL_LINE)
            dahlia_stacks.append([dahlia_group])
        dahlia_trace.add_cycle(i, CycleTrace(dahlia_stacks))
    return dahlia_trace


def create_and_write_adl_map(tracedata: TraceData, adl_mapping_file: str, out_dir: str):
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
            # We will create a Dahlia-specific trace
            dahlia_trace = create_dahlia_trace(tracedata, adl_map)
            flame.create_and_write_dahlia_flame_maps(tracedata, adl_mapping_file, out_dir)
            # adl_flat_map, adl_scaled_map = flame.create_flame_maps(
            #     dahlia_trace, FlameMapMode.ADL
            # )
            # flame.write_flame_map(adl_flat_map, adl_flat_flame_file)
            # flame.write_flame_map(adl_scaled_map, adl_scaled_flame_file)

            # print("writing Dahlia timeline")
            timeline.compute_adl_protobuf_timeline(dahlia_trace, out_dir)

        case Adl.PY:
            # for Calyx-py we can suffice with just using Calyx PTraces 
            adl_added_trace = tracedata.add_sourceloc_info(adl_map)

            adl_flat_map, adl_scaled_map = flame.create_flame_maps(
                adl_added_trace, FlameMapMode.ADL
            )
            flame.write_flame_map(adl_flat_map, adl_flat_flame_file)
            flame.write_flame_map(adl_scaled_map, adl_scaled_flame_file)

            mixed_flat_map, mixed_scaled_map = flame.create_flame_maps(
                adl_added_trace, FlameMapMode.MIXED
            )
            flame.write_flame_map(mixed_flat_map, mixed_flat_flame_file)
            flame.write_flame_map(mixed_scaled_map, mixed_scaled_flame_file)
