#!/usr/bin/env python3

import rpt
import sys
import re
import json

def find_row(table, colname, key):
    for row in table:
        if row[colname] == key:
            return row
    raise Error(f"{key} was not found in column: {colname}")

def futil_extract(parser):
    slice_logic = parser.get_table(re.compile(r'1\. Slice Logic'), 2)
    dsp_table = parser.get_table(re.compile(r'3. DSP'), 2)

    return {
        'LUT': find_row(slice_logic, 'Site Type', 'Slice LUTs*')['Used'],
        'DSP': find_row(dsp_table, 'Site Type', 'DSPs')['Used']
    }

def hls_extract(parser):
    summary_table = parser.get_table(re.compile(r'== Utilization Estimates'), 2)

    return {
        'LUT': find_row(summary_table, 'Name', 'Total')['LUT'],
        'DSP48': find_row(summary_table, 'Name', 'Total')['DSP48E']
    }

def main(style, filename):
    parser = rpt.RPTParser(filename)
    if style == 'futil':
        print(json.dumps(futil_extract(parser)))
    elif style == 'hls':
        print(json.dumps(hls_extract(parser)))

if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
