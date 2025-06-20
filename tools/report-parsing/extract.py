import json
import os
from pathlib import Path, PurePath
import re
import traceback
import logging as log

import rpt


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


def rpt_extract(file: PurePath):
    if not file.exists():
        log.error(f"RPT file {file} is missing")
        return None

    parser = rpt.RPTParser(file)

    # Optional asterisk at the end of table name here because in synthesis files
    # the name comes with an asterisk
    slice_logic = parser.get_table(re.compile(r"^\d+\. CLB Logic\*?$"), 2)
    bram_table = parser.get_table(re.compile(r"^\d+\. BLOCKRAM$"), 2)
    dsp_table = parser.get_table(re.compile(r"^\d+\. ARITHMETIC$"), 2)
    logic_type = "CLB"

    if not all([slice_logic, bram_table, dsp_table]):
        log.warning("Failed to find CLB logic tables, defaulting to older RPT format")
        slice_logic = parser.get_table(re.compile(r"^\d+\. Slice Logic\*?$"), 2)
        bram_table = parser.get_table(re.compile(r"^\d+\. Memory$"), 2)
        dsp_table = parser.get_table(re.compile(r"^\d+\. DSP$"), 2)
        logic_type = "Slice"

    if not all([slice_logic, bram_table, dsp_table]):
        log.error("Failed to extract resource information")
        return None

    # print(slice_logic, bram_table, dsp_table)
    lut = find_row(slice_logic, "Site Type", f"{logic_type} LUTs", False)
    if lut is None:
        # Try to find the LUTs with the asterisk for synthesis files
        lut = find_row(slice_logic, "Site Type", f"{logic_type} LUTs*", False)
    lut = safe_get(lut, "Used")
    reg = safe_get(
        find_row(slice_logic, "Site Type", f"{logic_type} Registers", False), "Used"
    )
    carry8 = safe_get(find_row(slice_logic, "Site Type", "CARRY8", False), "Used")
    f7_muxes = safe_get(find_row(slice_logic, "Site Type", "F7 Muxes", False), "Used")
    f8_muxes = safe_get(find_row(slice_logic, "Site Type", "F8 Muxes", False), "Used")
    f9_muxes = safe_get(find_row(slice_logic, "Site Type", "F9 Muxes", False), "Used")
    dsp = safe_get(find_row(dsp_table, "Site Type", "DSPs", False), "Used")
    brams = safe_get(find_row(bram_table, "Site Type", "Block RAM Tile", False), "Used")

    return {
        "summary": {
            "lut": to_int(lut),
            "dsp": to_int(dsp),
            "brams": to_int(brams),
            "registers": to_int(reg),
            "carry8": to_int(carry8),
            "f7_muxes": to_int(f7_muxes),
            "f8_muxes": to_int(f8_muxes),
            "f9_muxes": to_int(f9_muxes),
        },
        "tables": {
            "slice_logic": slice_logic,
            "bram_table": bram_table,
            "dsp_table": dsp_table,
        },
    }


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
        resource_info.update(
            {
                "synth": rpt_extract(synth_file),
                "impl": rpt_extract(util_file),
            }
        )

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
        log.error(f"File not found: {e.filename}")


if __name__ == "__main__":
    print(
        place_and_route_extract(
            Path("out"),
            "FutilBuild.runs",
            PurePath("impl_1", "main_utilization_placed.rpt"),
            PurePath("impl_1", "main_timing_summary_routed.rpt"),
            PurePath("synth_1", "main_utilization_synth.rpt"),
        )
    )
