import argparse
import json
from pathlib import Path

from AreaExtract.lib.parse.vivado import rpt_to_design_with_metadata
from AreaExtract.lib.parse.yosys import il_to_design_with_metadata


def main():
    parser = argparse.ArgumentParser(
        description=(
            "Parse FPGA synthesis reports into a Common Data Format.\n\n"
            "Supported origins:\n"
            "  - Vivado: single hierarchical .rpt file\n"
            "  - Yosys: .il (intermediate language) and .json (stat) file\n\n"
            "Output is a JSON serialization of the Common Data Format."
        ),
        formatter_class=argparse.RawTextHelpFormatter,
    )
    subparsers = parser.add_subparsers(dest="origin", required=True)
    vivado = subparsers.add_parser(
        "vivado",
        help="parse a Vivado utilization .rpt file",
    )
    vivado.add_argument(
        "rpt",
        type=Path,
        help="path to Vivado utilization report (.rpt)",
    )
    yosys = subparsers.add_parser(
        "yosys",
        help="parse Yosys IL and stat JSON files",
    )
    yosys.add_argument(
        "il",
        type=Path,
        help="path to Yosys IL file (.il)",
    )
    yosys.add_argument(
        "json",
        type=Path,
        help="path to Yosys stat file (.json)",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="optional output file for JSON (defaults to stdout)",
    )
    args = parser.parse_args()
    if args.origin == "vivado":
        design = rpt_to_design_with_metadata(args.rpt)
    elif args.origin == "yosys":
        design = il_to_design_with_metadata(args.il, args.json)
    else:
        parser.error("unknown origin")
    json_str = json.dumps(design, default=lambda o: o.__dict__, indent=2)
    if args.output:
        args.output.write_text(json_str)
    else:
        print(json_str)


if __name__ == "__main__":
    main()
