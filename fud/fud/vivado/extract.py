from . import rpt
import sys
import re
import json


def find_row(table, colname, key):
    for row in table:
        if row[colname] == key:
            return row
    raise Exception(f"{key} was not found in column: {colname}")


def to_int(s):
    if s == '-':
        return 0
    else:
        return int(s)


def file_contains(regex, filename):
    strings = re.findall(regex, filename.open().read())
    return len(strings) == 0


def rtl_component_extract(directory, name):
    try:
        with (directory / "synth_1" / "runme.log").open() as f:
            log = f.read()
            comp_usage = re.search(r'Start RTL Component Statistics(.*?)Finished RTL', log, re.DOTALL).group(1)
            a = re.findall('{} := ([0-9]*).*$'.format(name), comp_usage, re.MULTILINE)
            return sum(map(int, a))
    except Exception as e:
        print(e)
        print("RTL component log not found")
        return 0


def futil_extract(directory):
    directory = directory / "out" / "FutilBuild.runs"
    try:
        parser = rpt.RPTParser(directory / "impl_1" / "main_utilization_placed.rpt")
        slice_logic = parser.get_table(re.compile(r'1\. CLB Logic'), 2)
        dsp_table = parser.get_table(re.compile(r'4\. ARITHMETIC'), 2)
        meet_timing = file_contains(r'Timing constraints are not met.', directory / "impl_1" / "main_timing_summary_routed.rpt")

        return json.dumps({
            'lut': to_int(find_row(slice_logic, 'Site Type', 'CLB LUTs')['Used']),
            'dsp': to_int(find_row(dsp_table, 'Site Type', 'DSPs')['Used']),
            'meet_timing': int(meet_timing),
            'registers': rtl_component_extract(directory, 'Registers'),
            'muxes': rtl_component_extract(directory, 'Muxes')
        }, indent=2)
    except Exception as e:
        print(e)
        print("Synthesis files weren't found, skipping.", file=sys.stderr)


def hls_extract(directory):
    directory = directory / "benchmark.prj" / "solution1"
    try:
        parser = rpt.RPTParser(directory / "syn" / "report" / "kernel_csynth.rpt")
        summary_table = parser.get_table(re.compile(r'== Utilization Estimates'), 2)
        instance_table = parser.get_table(re.compile(r'\* Instance:'), 0)

        solution_data = json.load((directory / "solution1_data.json").open())
        latency = solution_data['ModuleInfo']['Metrics']['kernel']['Latency']

        total_row = find_row(summary_table, 'Name', 'Total')
        s_axi_row = find_row(instance_table, 'Instance', 'kernel_control_s_axi_U')

        return json.dumps({
            'total_lut': to_int(total_row['LUT']),
            'instance_lut': to_int(s_axi_row['LUT']),
            'lut': to_int(total_row['LUT']) - to_int(s_axi_row['LUT']),
            'dsp': to_int(total_row['DSP48E']) - to_int(s_axi_row['DSP48E']),
            'avg_latency': to_int(latency['LatencyAvg']),
            'best_latency': to_int(latency['LatencyBest']),
            'worst_latency': to_int(latency['LatencyWorst']),
        }, indent=2)
    except Exception as e:
        print(e)
        print("HLS files weren't found, skipping.", file=sys.stderr)
