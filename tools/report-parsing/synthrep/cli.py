from synthrep.extract import place_and_route_extract, hls_extract
from synthrep.rpt import RPTParser
from pathlib import Path, PurePath
import argparse
import re
import json


def summary(dir, top):
    print(
        place_and_route_extract(
            Path(dir, "FutilBuild.runs"),
            PurePath("impl_1", f"{top}_utilization_placed.rpt"),
            PurePath("impl_1", f"{top}_timing_summary_routed.rpt"),
            PurePath("synth_1", f"{top}_utilization_synth.rpt"),
        )
    )


def flatten_tree(tree, prefix=""):
    flat = {}
    for name, node in tree.items():
        fq_name = f"{prefix}.{name}" if prefix else name
        flat[fq_name] = {k: v for k, v in node.items() if k != "children"}
        if node.get("children"):
            flat.update(flatten_tree(node["children"], fq_name))
    return flat


def hierarchy_summary(dir):
    print(
        json.dumps(
            flatten_tree(create_tree(Path(dir, "hierarchical_utilization_placed.rpt"))),
            indent=2,
        )
    )


def hls_summary(dir, top):
    print(hls_extract(Path(dir), top))


def hls_impl_summary(dir, top):
    print(
        place_and_route_extract(
            Path(dir, "solution1", "impl", "verilog", "report"),
            PurePath(f"{top}_utilization_routed.rpt"),
            PurePath(f"{top}_timing_routed.rpt"),
            PurePath(f"{top}_utilization_synth.rpt"),
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


def main():
    parser = argparse.ArgumentParser(
        prog="synthrep",
        description="Utility to help with parsing and data visualization of Vivado reports.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)
    viz_parser = subparsers.add_parser("viz", help="create data visualizations")
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
        help="specify Vivado output directory (default: <mode dependent>)",
    )
    summary_parser.add_argument(
        "-m",
        "--mode",
        help="set summary mode (default: %(default)s)",
        choices=["utilization", "hierarchy", "hls", "hls-impl"],
        default="utilization",
    )
    summary_parser.add_argument(
        "--top",
        help="specify top-level module/function (default: %(default)s)",
        default="main",
    )
    args = parser.parse_args()
    match args.command:
        case "summary":
            match args.mode:
                case "utilization":
                    summary(args.directory or "out", args.top)
                case "hierarchy":
                    hierarchy_summary(args.directory or "out")
                case "hls":
                    hls_summary(args.directory or "benchmark.prj", args.top)
                case "hls-impl":
                    hls_impl_summary(args.directory or "benchmark.prj", args.top)
        case "viz":
            flamegraph_folded(args.filename, map[args.column])
