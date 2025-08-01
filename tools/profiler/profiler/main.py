import argparse
from datetime import datetime
import os
from profiler.visuals.utilization_plots import Plotter
import vcdvcd
import json

import profiler.adl_mapping as adl_mapping
import profiler.construct_trace as construct_trace
import profiler.preprocess as preprocess
from profiler.visuals import flame, timeline, stats

from profiler.classes import (
    CellMetadata,
    ControlMetadata,
    TraceData,
    ControlRegUpdateType,
    Utilization,
)


def setup_metadata(args):
    """
    Wrapper function to preprocess information to use in VCD reading.
    """
    cell_metadata: CellMetadata = preprocess.read_component_cell_names_json(
        args.cells_json
    )
    shared_cells_map: dict[str, dict[str, str]] = preprocess.read_shared_cells_map(
        args.shared_cells_json
    )
    enable_thread_data = preprocess.read_enable_thread_json(args.enable_par_tracks_json)
    if args.ctrl_mapping_file is not None:
        component_to_pos_to_loc_str = preprocess.read_ctrl_metadata_file(
            args.ctrl_mapping_file
        )
    else:
        component_to_pos_to_loc_str = None

    control_metadata: ControlMetadata = preprocess.setup_control_info(
        args.fsms_json,
        args.path_descriptors_json,
        component_to_pos_to_loc_str,
        cell_metadata,
    )
    # create tracedata object here so we can use it outside of converter
    tracedata: TraceData = TraceData()
    return (
        cell_metadata,
        control_metadata,
        tracedata,
        shared_cells_map,
        enable_thread_data,
    )


def process_vcd(
    cell_metadata: CellMetadata,
    shared_cells_map: dict[str, dict[str, str]],
    control_metadata: ControlMetadata,
    tracedata: TraceData,
    vcd_filename: str,
    utilization: Utilization | None = None,
):
    """
    Wrapper function to process the VCD file to produce a trace.
    control_reg_updates_per_cycle will be used by flame.create_simple_flame_graph(), hence we are returning it.
    """
    print(f"Start reading VCD: {datetime.now()}")
    converter = construct_trace.VCDConverter(cell_metadata, control_metadata, tracedata)
    vcdvcd.VCDVCD(vcd_filename, callbacks=converter)
    print(f"Start Postprocessing VCD: {datetime.now()}")

    converter.postprocess(shared_cells_map, control_metadata, utilization)
    (
        control_groups_trace,
        control_reg_updates_per_cycle,
    ) = converter.postprocess_control()
    converter.postprocess_cont()
    del converter
    tracedata.create_trace_with_control_groups(
        control_groups_trace, cell_metadata, control_metadata, utilization
    )
    print(f"End Postprocessing VCD: {datetime.now()}")
    print(f"End reading VCD: {datetime.now()}")

    return control_reg_updates_per_cycle


def create_visuals(
    cell_metadata: CellMetadata,
    control_metadata: ControlMetadata,
    tracedata: TraceData,
    enable_thread_metadata: dict[str, dict[str, int]],
    control_reg_updates_per_cycle: dict[int, ControlRegUpdateType],
    out_dir: str,
    flame_out: str,
    utilization_variable: str | None = None,
):
    """
    Wrapper function to compute statistics, write flame graphs, and write timeline view.
    """

    # create output directory for profiler results
    if not os.path.exists(out_dir):
        os.mkdir(out_dir)

    flame.create_simple_flame_graph(tracedata, control_reg_updates_per_cycle, out_dir)
    stats.write_group_stats(cell_metadata, tracedata, out_dir)
    stats.write_cell_stats(
        cell_metadata,
        control_metadata,
        tracedata,
        out_dir,
    )
    stats.write_par_stats(tracedata, out_dir)
    print(f"End writing stats: {datetime.now()}")

    # create flame graphs without control
    nc_flat_flame_map, nc_scaled_flame_map = flame.create_flame_maps(tracedata.trace)
    nc_flat_flame_file = os.path.join(out_dir, "nc-flat-flame.folded")
    flame.write_flame_maps(nc_flat_flame_map, nc_scaled_flame_map, out_dir, nc_flat_flame_file, "nc-scaled-flame.folded")

    flat_flame_map, scaled_flame_map = flame.create_flame_maps(
        tracedata.trace_with_control_groups
    )
    flame.write_flame_maps(flat_flame_map, scaled_flame_map, out_dir, flame_out)
    print(f"End writing flame graphs: {datetime.now()}")

    timeline.compute_protobuf_timeline(
        tracedata, cell_metadata, enable_thread_metadata, out_dir
    )
    timeline.compute_timeline(tracedata, cell_metadata, enable_thread_metadata, out_dir)
    print(f"End writing timeline view: {datetime.now()}")

    if utilization_variable:
        p = Plotter(tracedata.trace_with_control_groups)
        p.run_all(utilization_variable, out_dir)


def parse_args():
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
    parser.add_argument(
        "enable_par_tracks_json",
        help="Records statically assigned thread ids for control enables",
    )
    parser.add_argument(
        "path_descriptors_json",
        help="Records path descriptors for enables and control nodes",
    )
    parser.add_argument("out_dir", help="Output directory")
    parser.add_argument("flame_out", help="Output file for flattened flame graph")
    parser.add_argument(
        "--utilization-report-json",
        dest="utilization_report_json",
        help="utilization report json file",
    )
    parser.add_argument(
        "--utilization-variable",
        dest="utilization_variable",
        help="utilization variable to visualize (default: %(default)s)",
        default="ff",
        choices=["ff", "lut", "llut", "lutram"],
    )
    parser.add_argument(
        "--ctrl-pos-file",
        dest="ctrl_mapping_file",
        help="json containing components to the pos and locations of their ctrl nodes",
    )
    parser.add_argument(
        "--adl-mapping-file", dest="adl_mapping_file", help="adl mapping file"
    )
    parser.add_argument(
        "--print-trace-threshold",
        dest="print_trace_threshold",
        type=int,
        default=0,
        help="Print the trace to stdout if less than or equal to specified number of cycles",
    )
    args = parser.parse_args()
    return args


def main():
    args = parse_args()
    print(f"Start time: {datetime.now()}")

    (
        cell_metadata,
        control_metadata,
        tracedata,
        shared_cells_map,
        enable_thread_metadata,
    ) = setup_metadata(args)

    utilization: Utilization | None = None
    utilization_variable: str | None = None

    if args.utilization_report_json is not None:
        print("Utilization report mode enabled.")
        with open(args.utilization_report_json) as f:
            utilization = Utilization(json.load(f))
            varmap = {
                "ff": "FFs",
                "lut": "Total LUTs",
                "llut": "Logic LUTs",
                "lutram": "LUTRAMs",
            }
            utilization_variable = varmap[args.utilization_variable]

    control_reg_updates_per_cycle: dict[int, ControlRegUpdateType] = process_vcd(
        cell_metadata,
        shared_cells_map,
        control_metadata,
        tracedata,
        args.vcd_filename,
        utilization,
    )

    tracedata.print_trace(threshold=args.print_trace_threshold, ctrl_trace=True)
    if utilization:
        print(
            f"Unaccessed utilization values: {', '.join(utilization.get_unaccessed())}"
        )

    create_visuals(
        cell_metadata,
        control_metadata,
        tracedata,
        enable_thread_metadata,
        control_reg_updates_per_cycle,
        args.out_dir,
        args.flame_out,
        utilization_variable,
    )

    if args.adl_mapping_file is not None:  # emit ADL flame graphs.
        adl_mapping.create_and_write_adl_map(
            tracedata, args.adl_mapping_file, args.out_dir
        )

    print(f"End time: {datetime.now()}")
