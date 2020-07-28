# Vivado Experiments
This document describes how to run experiments comparing Futil generated SystemVerilog
that is synthesized using `vivado` and Dahlia generated Vivado C++ that is synthesized
with `vivado_hls`.

## Scripts

### `vivado.sh`
This script is in charge of invoking `vivado` or `vivado_hls`
on a server where this is installed.

**Arguments**: `./vivado.sh <hls|futil> src_file dest_dir`
 1) Type: This is either the string `'hls'` or the string `'futil'`.
    - `'futil'` means that the source file is expected to be `.sv` file
    generated from the FuTIL compiler. `vivado -mode batch -source synth.tcl`
    will be run to synthesis it.
    - `'hls'` means that the source file is expected to be a Vivado HLS `.cpp` file.
    `vivado_hls -f hls.tcl` will be run to synthesis it.
 2) Source File: This is the `.sv` or `.cpp` source file that will be synthesized.
 3) Dest Directory: The local directory that the results of synthesis will be copied to.

### `compare.sh`
A script that runs a `.fuse` Dahlia file through both Futil compilation and Vivado HLS synthesis.

**Arguments**: `./compare.sh <.fuse src file> <benchmark name> <result directory>`

### `run_all.sh`
Uses `parallel` to run `./compare.sh` on all the benchmarks listed in the passed in file.

**Arguments**: `./run_all.sh <benchmark_file> <args to parallel>`

### `extract.py`
Python script that extracts various resource numbers into a json file.

### `rpt.sh`
Python script for extracting JSON files.
