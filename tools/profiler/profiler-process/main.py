import argparse
from datetime import datetime
import os
import vcdvcd
from dataclasses import dataclass

import adl_mapping
import construct_trace
import preprocess
from visuals import flame, tree, timeline, stats

from classes import TraceData

@dataclass
class PreprocessedInfo:
    main_shortname: str

def main(args):
    print(f"Start time: {datetime.now()}")
    # Preprocess information to use in VCD reading
    cell_metadata = preprocess.preprocess_cell_infos(args.cells_json, args.shared_cells_json)
    shared_cells_map = preprocess.read_shared_cells_map(args.shared_cells_json)
    control_metadata = preprocess.read_tdcc_file(args.tdcc_json_file, cell_metadata)
    # create tracedata object here so we can use it outside of converter
    tracedata = TraceData()
    # FIXME: just create everything in create_timeline(), remove below commented code
    # # create dict of fsms outside the converter so they are preserved.
    # fsm_events = {
    #     fsm: [{"name": str(0), "cat": "fsm", "ph": "B", "ts": 0}]
    #     for fsm in fully_qualified_fsms
    # }  # won't be fully filled in until create_timeline()
    print(f"Start reading VCD: {datetime.now()}")
    converter = construct_trace.VCDConverter(
        cell_metadata,
        control_metadata,
        tracedata
    )
    vcdvcd.VCDVCD(args.vcd_filename, callbacks=converter)
    main_fullname = converter.main_component
    print(f"Start Postprocessing VCD: {datetime.now()}")

    trace, trace_classified, cell_to_active_cycles = converter.postprocess(
        shared_cells_map
    )  # trace contents: cycle # --> list of stacks, trace_classified is a list: cycle # (indices) --> # useful stacks
    (
        control_groups_trace,
        control_groups_summary,
        control_reg_updates,
        control_reg_updates_per_cycle,
    ) = converter.postprocess_control()
    cell_to_ordered_pars = construct_trace.order_pars(
        cell_to_pars, par_to_children, reverse_par_dep_info
    )
    trace_with_pars = construct_trace.add_par_to_trace(
        trace,
        control_groups_trace,
        cell_to_ordered_pars,
        cell_to_groups_to_par_parent,
        main_shortname,
    )
    print(f"End Postprocessing VCD: {datetime.now()}")
    print(f"End reading VCD: {datetime.now()}")
    del converter

    # debug printing for programs that are less than print_trace_threshold (optional arg; default 0) cycles long
    if len(trace) < args.print_trace_threshold:
        for i in trace_with_pars:
            print(i)
            for stack in trace_with_pars[i]:
                print(f"\t{stack}")

    if not os.path.exists(args.out_dir):
        os.mkdir(args.out_dir)
    cats_to_cycles = flame.create_simple_flame_graph(
        trace_classified, control_reg_updates_per_cycle, args.out_dir
    )
    print(f"End creating simple flame graph: {datetime.now()}")
    stats.write_cell_stats(
        cell_to_active_cycles,
        cats_to_cycles,
        cells_to_components,
        component_to_num_fsms,
        len(trace),
        args.out_dir,
    )
    stats.write_par_stats(
        control_groups_summary, cats_to_cycles, trace_with_pars, main_shortname, args.out_dir
    )
    print(f"End writing cell stats: {datetime.now()}")
    tree_dict, path_dict = tree.create_tree(trace)
    path_to_edges, all_edges = tree.create_edge_dict(path_dict)

    tree.create_aggregate_tree(trace, args.out_dir, tree_dict, path_dict)
    tree.create_tree_rankings(
        trace, tree_dict, path_dict, path_to_edges, all_edges, args.out_dir
    )
    flat_flame_map, scaled_flame_map = flame.create_flame_maps(trace_with_pars)
    flame.write_flame_maps(flat_flame_map, scaled_flame_map, args.out_dir, args.flame_out)

    timeline.compute_timeline(
        trace, fsm_events, control_reg_updates, main_fullname, args.out_dir
    )

    if args.adl_mapping_file is not None:  # emit ADL flame graphs.
        create_adl_visuals(args.adl_mapping_file, args.out_dir, flat_flame_map, scaled_flame_map)

    print(f"End time: {datetime.now()}")


def create_adl_visuals(adl_mapping_file, out_dir, flat_flame_map, scaled_flame_map):
    print("Computing ADL flames...")
    adl_flat_flame, mixed_flat_flame = adl_mapping.convert_flame_map(
        flat_flame_map, adl_mapping_file
    )
    adl_scaled_flame, mixed_scaled_flame = adl_mapping.convert_flame_map(
        scaled_flame_map, adl_mapping_file
    )
    adl_flat_flame_file = os.path.join(out_dir, "adl-flat-flame.folded")
    adl_scaled_flame_file = os.path.join(out_dir, "adl-scaled-flame.folded")
    flame.write_flame_maps(
        adl_flat_flame,
        adl_scaled_flame,
        out_dir,
        adl_flat_flame_file,
        adl_scaled_flame_file,
    )

    mixed_flat_flame_file = os.path.join(out_dir, "mixed-flat-flame.folded")
    mixed_scaled_flame_file = os.path.join(out_dir, "mixed-scaled-flame.folded")
    flame.write_flame_maps(
        mixed_flat_flame,
        mixed_scaled_flame,
        out_dir,
        mixed_flat_flame_file,
        mixed_scaled_flame_file,
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Analyze instrumented VCD file and generate initial files for visualizations"
    )
    parser.add_argument("vcd_filename", help="Instrumented VCD file")
    parser.add_argument(
        "cells_json", help="File mapping components to the cells that they contain."
    )
    parser.add_argument(
        "fsms_json",
        help='Run the Calyx compiler with -x tdcc:dump-fsm-json="<FILENAME>" to obtain the file.',
    )
    parser.add_argument(
        "shared_cells_json",
        help="Records cells that are shared during cell-share pass.",
    )
    parser.add_argument("out_dir", help="Output directory")
    parser.add_argument("flame_out", help="Flame")
    parser.add_argument(
        "--adl-mapping-file", dest="adl_mapping_file", help="adl mapping file"
    )
    parser.add_argument(
        "--print-trace-threshold",
        dest="print_trace_threshold",
        action="store_true",
        default=0,
        help="Print the trace to stdout if less than or equal to specified number of cycles",
    )
    args = parser.parse_args()
    main(args)
