import json
import os
from pathlib import Path, PurePath
import re
import traceback
import logging as log

from fud import errors

from . import rpt


def find_row(table, colname, key, certain=True):
    for row in table:
        if row[colname] == key:
            return row
    if certain:
        raise Exception(f"{key} was not found in column: {colname}")

    return None


def safe_get(d, key):
    if d is not None and key in d:
        return d[key]
    return -1


def to_int(s):
    if s == "-":
        return 0
    return int(s)


def file_contains(regex, filename):
    strings = re.findall(regex, filename.open().read())
    return len(strings) == 0


def rtl_component_extract(file: Path, name: str):
    try:
        with file.open() as f:
            log = f.read()
            comp_usage = re.search(
                r"Start RTL Component Statistics(.*?)Finished RTL", log, re.DOTALL
            ).group(1)
            a = re.findall("{} := ([0-9]*).*$".format(name), comp_usage, re.MULTILINE)
            return sum(map(int, a))
    except Exception as e:
        print(e)
        print("RTL component log not found")
        return 0


def place_and_route_extract(
    directory: Path,
    files_root: str,
    utilization_file: PurePath,
    timing_file: PurePath,
    synthesis_file: PurePath,
):
    # Search for the given root directory
    for root, dirs, _ in os.walk(directory):
        for d in dirs:
            if d == files_root:
                directory = Path(os.path.join(root, d))
                break

    util_file = directory / utilization_file
    synth_file = directory / synthesis_file
    timing_file = directory / timing_file

    # The resource information is extracted first for the implementation files, and
    # then for the synthesis files. This is done separately in case users want to
    # solely use one or the other.
    resource_info = {}

    # Extract utilization information
    try:
        if util_file.exists():
            impl_parser = rpt.RPTParser(util_file)
            slice_logic = impl_parser.get_table(re.compile(r"1\. CLB Logic"), 2)
            dsp_table = impl_parser.get_table(re.compile(r"4\. ARITHMETIC"), 2)

            clb_lut = to_int(find_row(slice_logic, "Site Type", "CLB LUTs")["Used"])
            clb_reg = to_int(
                find_row(slice_logic, "Site Type", "CLB Registers")["Used"]
            )
            carry8 = to_int(find_row(slice_logic, "Site Type", "CARRY8")["Used"])
            f7_muxes = to_int(find_row(slice_logic, "Site Type", "F7 Muxes")["Used"])
            f8_muxes = to_int(find_row(slice_logic, "Site Type", "F8 Muxes")["Used"])
            f9_muxes = to_int(find_row(slice_logic, "Site Type", "F9 Muxes")["Used"])
            resource_info.update(
                {
                    "lut": to_int(
                        find_row(slice_logic, "Site Type", "CLB LUTs")["Used"]
                    ),
                    "dsp": to_int(find_row(dsp_table, "Site Type", "DSPs")["Used"]),
                    "registers": rtl_component_extract(synth_file, "Registers"),
                    "muxes": rtl_component_extract(synth_file, "Muxes"),
                    "clb_registers": clb_reg,
                    "carry8": carry8,
                    "f7_muxes": f7_muxes,
                    "f8_muxes": f8_muxes,
                    "f9_muxes": f9_muxes,
                    "clb": clb_lut + clb_reg + carry8 + f7_muxes + f8_muxes + f9_muxes,
                }
            )
        else:
            log.error(f"Utilization implementation file {util_file} is missing")

    except Exception:
        log.error(traceback.format_exc())
        log.error("Failed to extract utilization information")

    # Get timing information
    if not timing_file.exists():
        log.error(f"Timing file {timing_file} is missing")
    else:
        meet_timing = file_contains(r"Timing constraints are not met.", timing_file)
        resource_info.update(
            {
                "meet_timing": int(meet_timing),
            }
        )

        # Extract timing information
        timing_parser = rpt.RPTParser(timing_file)
        slack_info = timing_parser.get_bare_table(re.compile(r"Design Timing Summary"))
        if slack_info is None:
            log.error("Failed to extract slack information")
        resource_info.update({"worst_slack": float(safe_get(slack_info, "WNS(ns)"))})

        period_info = timing_parser.get_bare_table(re.compile(r"Clock Summary"))
        if slack_info is None:
            log.error("Failed to extract clock information")
        resource_info.update({"period": float(safe_get(period_info, "Period(ns)"))})
        resource_info.update(
            {"frequency": float(safe_get(period_info, "Frequency(MHz)"))}
        )

    # Extraction for synthesis files.
    try:
        if not synth_file.exists():
            log.error(f"Synthesis file {synth_file} is missing")
        else:
            synth_parser = rpt.RPTParser(synth_file)
            cell_usage_tbl = synth_parser.get_table(
                re.compile(r"Report Cell Usage:"), 0
            )
            cell_lut1 = find_row(cell_usage_tbl, "Cell", "LUT1", False)
            cell_lut2 = find_row(cell_usage_tbl, "Cell", "LUT2", False)
            cell_lut3 = find_row(cell_usage_tbl, "Cell", "LUT3", False)
            cell_lut4 = find_row(cell_usage_tbl, "Cell", "LUT4", False)
            cell_lut5 = find_row(cell_usage_tbl, "Cell", "LUT5", False)
            cell_lut6 = find_row(cell_usage_tbl, "Cell", "LUT6", False)
            cell_fdre = find_row(cell_usage_tbl, "Cell", "FDRE", False)
            uram_usage = find_row(cell_usage_tbl, "Cell", "URAM288", False)

            resource_info.update(
                {
                    "uram": to_int(safe_get(uram_usage, "Count")),
                    "cell_lut1": to_int(safe_get(cell_lut1, "Count")),
                    "cell_lut2": to_int(safe_get(cell_lut2, "Count")),
                    "cell_lut3": to_int(safe_get(cell_lut3, "Count")),
                    "cell_lut4": to_int(safe_get(cell_lut4, "Count")),
                    "cell_lut5": to_int(safe_get(cell_lut5, "Count")),
                    "cell_lut6": to_int(safe_get(cell_lut6, "Count")),
                    "cell_fdre": to_int(safe_get(cell_fdre, "Count")),
                }
            )
    except Exception:
        log.error(traceback.format_exc())
        log.error("Failed to extract synthesis information")

    return json.dumps(resource_info, indent=2)


def hls_extract(directory: Path, top: str):
    # Search for directory named benchmark.prj
    for root, dirs, _ in os.walk(directory):
        for d in dirs:
            if d == "benchmark.prj":
                directory = Path(os.path.join(root, d))
                break

    directory = directory / "solution1"

    try:
        parser = rpt.RPTParser(directory / "syn" / "report" / f"{top}_csynth.rpt")
        summary_table = parser.get_table(re.compile(r"== Utilization Estimates"), 2)
        instance_table = parser.get_table(re.compile(r"\* Instance:"), 0)

        solution_data = json.load((directory / "solution1_data.json").open())
        latency = solution_data["ModuleInfo"]["Metrics"][top]["Latency"]

        total_row = find_row(summary_table, "Name", "Total")
        s_axi_row = find_row(instance_table, "Instance", f"{top}_control_s_axi_U")

        return json.dumps(
            {
                "total_lut": to_int(total_row["LUT"]),
                "instance_lut": to_int(s_axi_row["LUT"]),
                "lut": to_int(total_row["LUT"]) - to_int(s_axi_row["LUT"]),
                "dsp": to_int(total_row["DSP48E"]) - to_int(s_axi_row["DSP48E"]),
                "avg_latency": to_int(latency["LatencyAvg"]),
                "best_latency": to_int(latency["LatencyBest"]),
                "worst_latency": to_int(latency["LatencyWorst"]),
            },
            indent=2,
        )
    except FileNotFoundError as e:
        raise errors.MissingFile(e.filename)
