import argparse
from datetime import datetime
import os
from profiler.visuals.plots import Plotter
import vcdvcd
import json
import pandas as pd
import plotly.express as px
from collections import Counter

import profiler.adl_mapping as adl_mapping
import profiler.construct_trace as construct_trace
import profiler.preprocess as preprocess
from profiler.visuals import flame, timeline, stats

from profiler.classes import (
    CellMetadata,
    ControlMetadata,
    TraceData,
    ControlRegUpdateType,
    UtilizationCycleTrace,
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
    control_metadata: ControlMetadata = preprocess.read_tdcc_file(
        args.fsms_json, cell_metadata
    )
    # create tracedata object here so we can use it outside of converter
    tracedata: TraceData = TraceData()
    return cell_metadata, shared_cells_map, control_metadata, tracedata


def process_vcd(
    cell_metadata: CellMetadata,
    shared_cells_map: dict[str, dict[str, str]],
    control_metadata: ControlMetadata,
    tracedata: TraceData,
    vcd_filename: str,
    utilization: dict[str, dict] | None = None,
):
    """
    Wrapper function to process the VCD file to produce a trace.
    control_reg_updates_per_cycle will be used by flame.create_simple_flame_graph(), hence we are returning it.
    """
    print(f"Start reading VCD: {datetime.now()}")
    converter = construct_trace.VCDConverter(cell_metadata, control_metadata, tracedata)
    vcdvcd.VCDVCD(vcd_filename, callbacks=converter)
    print(f"Start Postprocessing VCD: {datetime.now()}")

    converter.postprocess(shared_cells_map, utilization)
    (
        control_groups_trace,
        control_reg_updates_per_cycle,
    ) = converter.postprocess_control()
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

    flat_flame_map, scaled_flame_map = flame.create_flame_maps(
        tracedata.trace_with_control_groups
    )
    flame.write_flame_maps(flat_flame_map, scaled_flame_map, out_dir, flame_out)
    print(f"End writing flame graphs: {datetime.now()}")

    timeline.compute_timeline(tracedata, cell_metadata, out_dir)
    print(f"End writing timeline view: {datetime.now()}")

    if utilization_variable:
        p = Plotter(tracedata.trace)
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
    parser.add_argument("out_dir", help="Output directory")
    parser.add_argument("flame_out", help="Flame")
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

    cell_metadata, shared_cells_map, control_metadata, tracedata = setup_metadata(args)

    utilization: dict[str, dict] | None = None
    utilization_variable: str | None = None

    if args.utilization_report_json is not None:
        print("Utilization report mode enabled.")
        with open(args.utilization_report_json) as f:
            utilization = json.load(f)
            map = {
                "ff": "FFs",
                "lut": "Total LUTs",
                "llut": "Logic LUTs",
                "lutram": "LUTRAMs",
            }
            utilization_variable = map[args.utilization_variable]

    control_reg_updates_per_cycle: dict[int, ControlRegUpdateType] = process_vcd(
        cell_metadata,
        shared_cells_map,
        control_metadata,
        tracedata,
        args.vcd_filename,
        utilization,
    )

    tracedata.print_trace(threshold=args.print_trace_threshold, ctrl_trace=True)

    create_visuals(
        cell_metadata,
        control_metadata,
        tracedata,
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
