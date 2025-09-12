import os

from profiler.visuals import flame
from profiler.classes.adl import AdlMap
from profiler.classes.tracedata import FlameMapMode, TraceData


def create_and_write_adl_map(tracedata: TraceData, adl_mapping_file: str, out_dir: str):
    """
    Creates ADL and Mixed (ADL + Calyx) versions of flame graph maps.
    """
    print(f"Creating ADL visuals from adl_map: {adl_mapping_file}")

    adl_map = AdlMap(adl_mapping_file)
    adl_added_trace = tracedata.add_sourceloc_info(adl_map)

    adl_flat_flame_file = os.path.join(out_dir, "adl-flat-flame.folded")
    adl_scaled_flame_file = os.path.join(out_dir, "adl-scaled-flame.folded")
    mixed_flat_flame_file = os.path.join(out_dir, "mixed-flat-flame.folded")
    mixed_scaled_flame_file = os.path.join(out_dir, "mixed-scaled-flame.folded")

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
