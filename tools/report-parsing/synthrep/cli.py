from synthrep.extract import place_and_route_extract
from synthrep.rpt import RPTParser
from pathlib import Path, PurePath
import pandas as pd
import plotly.express as px
import argparse
import re


def summary(dir):
    print(
        place_and_route_extract(
            Path(dir),
            "FutilBuild.runs",
            PurePath("impl_1", "main_utilization_placed.rpt"),
            PurePath("impl_1", "main_timing_summary_routed.rpt"),
            PurePath("synth_1", "main_utilization_synth.rpt"),
        )
    )


def create_tree(filename):
    rpt_file = Path(filename)
    parser = RPTParser(rpt_file)
    table = parser.get_table(
        re.compile(r"^\d+\. Utilization by Hierarchy$"), 2, preserve_indent=True
    )
    tree = parser.build_hierarchy_tree(table)
    return tree


def flamegraph_folded(filename, val):
    tree = create_tree(filename)
    print(RPTParser.generate_folded(tree, val).rstrip())


def plotly_viz(filename, fn, val, verbose=False):
    tree = create_tree(filename)
    flat = RPTParser.generate_flattened(tree, val)
    pd.set_option("display.max_rows", None)
    df = pd.DataFrame(flat)
    if verbose:
        print(df)
    fig = fn(
        df,
        names="label",
        parents="parent",
        values="value",
        ids="id",
    )
    if fn == px.treemap:
        fig.update_traces(marker=dict(cornerradius=5))
    fig.show()


def main():
    parser = argparse.ArgumentParser(
        prog="synthrep",
        description="Utility to help with parsing and data visualization of Vivado reports.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)
    viz_parser = subparsers.add_parser("viz", help="create data visualizations")
    viz_parser.add_argument(
        "-t",
        "--type",
        help="set type of visualization (default: %(default)s)",
        choices=["flamegraph", "treemap", "sunburst", "icicle"],
        default="treemap",
    )
    viz_parser.add_argument(
        "-f",
        "--filename",
        help="specify area report file (default: %(default)s)",
        default="out/hierarchical_utilization_placed.rpt",
    )
    viz_parser.add_argument(
        "-c",
        "--column",
        help="set column to visualize (default: %(default)s)",
        default="ff",
        choices=["ff", "lut", "llut", "lutram"],
    )
    viz_parser.add_argument(
        "-v",
        "--verbose",
        help="enable verbose mode (default: %(default)s)",
        action="store_true",
    )
    map = {"ff": "FFs", "lut": "Total LUTs", "llut": "Logic LUTs", "lutram": "LUTRAMs"}
    summary_parser = subparsers.add_parser("summary", help="output JSON summary")
    summary_parser.add_argument(
        "-d",
        "--directory",
        help="specify Vivado output directory (default: %(default)s)",
        default="out",
    )
    args = parser.parse_args()
    match args.command:
        case "summary":
            summary(args.directory)
        case "viz":
            match args.type:
                case "flamegraph":
                    flamegraph_folded(args.filename, map[args.column])
                case "treemap":
                    plotly_viz(
                        args.filename, px.treemap, map[args.column], args.verbose
                    )
                case "sunburst":
                    plotly_viz(
                        args.filename, px.sunburst, map[args.column], args.verbose
                    )
                case "icicle":
                    plotly_viz(args.filename, px.icicle, map[args.column], args.verbose)
