# AreaExtract

AreaExtract is a tool that replaces previous technology-specific frontends for
the Calyx profiler, Petal. It offers a combined frontend for several sources of
area data for accelerator designs, and outputs processed data in a common data
format that is parseable by Petal. It can also output area-only visualizations
for a specific variable. Currently, the following technologies are supported:

- Vivado, as hierarchical area synthesis reports
- Yosys, as both IL and statistics files

It can be installed using `uv pip install .` from the outermost `AreaExtract`
directory.

## Usage

```
$ aext -h 
usage: aext [-h] [-o OUTPUT] [-v VISUAL] [-c COLUMN] {vivado,yosys} ...

Parse FPGA synthesis reports into a Common Data Format.

Supported origins:
  - Vivado: single hierarchical .rpt file
  - Yosys: .il (intermediate language) and .json (stat) file

Supported outputs:
  - CDF: JSON serialization of the Common Data Format.
  - Visualizations: HTML hierarchical area-only visualizations.

positional arguments:
  {vivado,yosys}
    vivado             parse a Vivado utilization .rpt file
    yosys              parse Yosys IL and stat JSON files

options:
  -h, --help           show this help message and exit
  -o, --output OUTPUT  optional output file for JSON (defaults to stdout)
  -v, --visual VISUAL  save visualizations to folder (not done by default)
  -c, --column COLUMN  column to visualize (defaults to 'ff' for Vivado, 'width' for Yosys)
```

## Obtaining area data

This section provides instructions to obtain area data for designs from
supported technologies, to use as input for AreaExtract.

### Vivado

The simplest way to obtain a hierarchical area RPT file is to use Fud2 to run
synthesis on a Calyx design:

```
fud2 <design>.futil --to area-report > <report>.rpt
```

Alternatively, it is possible to use Fud2 to obtain a synthesis-ready Verilog
file, and then use Vivado directly to conduct synthesis. The relevant TCL
command for Vivado is:

```
report_utilization -hierarchical -file <report>.rpt
```

### Yosys

The synthesizable Verilog file can be obtained with:

```
fud2 <design>.futil --through calyx-to-synth-verilog > <rtl>.verilog
```

Then, using the OSS-CAD suite, IL and statistics files can be obtained as follows:

```
yosys -p "read_verilog -sv <VERILOG_FILE>.sv; hierarchy -top main; opt; write_rtlil <IL_FILE>.il; tee -o <STAT_FILE>.json stat -json"
```

It is also possible to pass Liberty files to [the `stat` command](https://yosyshq.readthedocs.io/projects/yosys/en/0.47/cmd/stat.html)
through the flag `-liberty <file>`.
