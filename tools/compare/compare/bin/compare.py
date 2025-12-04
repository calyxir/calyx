import argparse
import csv
import os
import sys
import json
from pathlib import Path, PurePath
from AreaExtract.lib.parse.vivado import rpt_to_design_with_metadata
from synthrep.extract import place_and_route_extract
import subprocess
from tabulate import tabulate


def main():
    parser = argparse.ArgumentParser(
        description=(
            "Compare data from simulation and synthesis for multiple Calyx files.\n\n"
            "Input is a CSV table with the following columns:\n"
            "  - DESIGN: design file name (e.g. 'example.futil')\n"
            "  - COMP_SIM: whether to compare simulated cycle counts (e.g. 'True')\n"
            "  - SIM_DATA: data file for simulation (e.g. 'example.data'), optional if no simulation\n"
            "  - COMP_SYNTH: whether to compare a synthesis variable (e.g. 'True')\n"
            "  - SYNTH_VAR: synthesis area variable to compare (e.g. 'ff'), optional if no synthesis\n"
            "  - SYNTH_PERIOD: clock period for synthesis (e.g. 7.00), optional if no synthesis\n\n"
            "Output is a CSV table with the following columns:\n"
            "  - DESIGN: design file name\n"
            "  - SIM_CYCLES: number of cycles in simulation\n"
            "  - SYNTH_STATUS: whether the design passed simulation\n"
            "  - SYNTH_AREA: value of area variable\n"
            "  - EXEC_TIME: calculated execution time with simulation and synthesis"
        ),
        formatter_class=argparse.RawTextHelpFormatter,
    )
    parser.add_argument("input", type=Path, help="path to input CSV")
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        default=False,
        help="log output to the terminal",
    )
    parser.add_argument(
        "-p",
        "--pretty-print",
        action="store_true",
        default=False,
        help="format output as a visual table rather than a CSV",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="optional output file (defaults to stdout)",
    )
    args = parser.parse_args()
    with open(args.input, newline="") as csvfile:
        table = csv.DictReader(csvfile)
        out = []
        out_dir = "comp-out"
        out_cols = [
            "DESIGN",
            "SIM_CYCLES",
            "SYNTH_STATUS",
            "SYNTH_AREA",
            "EXEC_TIME",
        ]
        try:
            os.mkdir(out_dir)
        except FileExistsError:
            if args.verbose:
                print("Overwriting existing output directory.")
        for row in table:
            out_row = {c: "" for c in out_cols}
            out.append(out_row)
            out_row["DESIGN"] = row["DESIGN"]
            run_dir = f"{out_dir}/run{len(out)}"
            if row["COMP_SIM"].lower() == "true":
                subprocess.run(
                    [
                        "fud2",
                        row["DESIGN"],
                        "-s",
                        f"sim.data={row['SIM_DATA']}",
                        "--to",
                        "flamegraph",
                        "--through",
                        "profiler",
                        "--dir",
                        run_dir,
                    ],
                    stdout=sys.stdout if args.verbose else subprocess.DEVNULL,
                )
                with open(Path(run_dir, "profiler-out", "total_cycles.txt")) as cycles:
                    out_row["SIM_CYCLES"] = int(cycles.read())
            if row["COMP_SYNTH"].lower() == "true":
                device_path = Path(run_dir, "default.xdc")
                with open(device_path, "w") as xdc:
                    xdc.write(
                        f"create_clock -period {row['SYNTH_PERIOD']} -name clk [get_ports clk]\n"
                    )
                subprocess.run(
                    [
                        "fud2",
                        row["DESIGN"],
                        "-s",
                        f"synth-verilog.constraints={device_path.resolve()}",
                        "--to",
                        "area-report",
                        "--dir",
                        run_dir,
                    ],
                    stdout=sys.stdout if args.verbose else subprocess.DEVNULL,
                )
                design = rpt_to_design_with_metadata(
                    Path(run_dir, "out", "hierarchical_utilization_placed.rpt")
                )
                out_row["SYNTH_AREA"] = design.design["main"].rsrc[row["SYNTH_VAR"]]
                summary = json.loads(
                    place_and_route_extract(
                        Path(run_dir, "out", "FutilBuild.runs"),
                        PurePath("impl_1", "main_utilization_placed.rpt"),
                        PurePath("impl_1", "main_timing_summary_routed.rpt"),
                        PurePath("synth_1", "main_utilization_synth.rpt"),
                    )
                )
                out_row["SYNTH_STATUS"] = bool(summary["meet_timing"])
            if (
                row["SYNTH_PERIOD"]
                and out_row["SIM_CYCLES"]
                and out_row["SYNTH_STATUS"]
            ):
                out_row["EXEC_TIME"] = (
                    float(row["SYNTH_PERIOD"]) * out_row["SIM_CYCLES"]
                )

        if args.pretty_print:
            print(tabulate(out, headers="keys", tablefmt="grid"))
        else:
            outfile = open(args.output, "w", newline="") if args.output else sys.stdout
            writer = csv.DictWriter(outfile, fieldnames=out[0].keys())
            writer.writeheader()
            writer.writerows(out)

            if outfile is not sys.stdout:
                outfile.close()


if __name__ == "__main__":
    main()
